use std::cmp;

use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::ToTokens;
use syn::{
    braced,
    parse::{Parse, ParseStream},
    parse2, parse_quote,
    punctuated::Punctuated,
    token::{Comma, Paren},
    visit_mut::{visit_expr_mut, visit_expr_struct_mut, VisitMut},
    Attribute, Expr, ExprPath, ExprStruct, FieldValue, Member, Path, Token, TypePath,
};

use crate::utils::resolve_agui_path;

struct BuildVisitor {}

impl BuildVisitor {}

impl VisitMut for BuildVisitor {
    fn visit_expr_mut(&mut self, i: &mut Expr) {
        visit_expr_mut(self, i);
    }

    fn visit_expr_struct_mut(&mut self, struct_expr: &mut ExprStruct) {
        visit_expr_struct_mut(self, struct_expr);

        // We have to ignore structs created with ::, since it may be an enum struct, which don't support
        // functional record updates
        if struct_expr.path.segments.len() > 1 {
            return;
        }

        // Add `..Default::default()` to structs that don't have a `..rest` initializer
        if struct_expr.dot2_token.is_none() && struct_expr.rest.is_none() {
            // Make sure fields have trailing comma
            if !struct_expr.fields.empty_or_trailing() {
                struct_expr.fields.push_punct(parse_quote! {,});
            }

            // Add the ..Default::default()
            struct_expr.dot2_token = Some(parse_quote! {..});
            struct_expr.rest = Some(Box::new(parse_quote! {
                ::core::default::Default::default()
            }));
        }
    }

    fn visit_field_value_mut(&mut self, value: &mut FieldValue) {
        // If there is a colon token, we cannot add `.into()`
        if value.colon_token.is_some() {
            if let Expr::Array(mut array_expr) = value.expr.clone() {
                self.visit_expr_array_mut(&mut array_expr);

                let count = array_expr.elems.len();

                let mut inner = quote::quote! {};

                for expr in array_expr.elems {
                    inner.extend(quote::quote! {
                        vec.push(#expr.into());
                    });
                }

                value.expr = Expr::MethodCall(parse_quote! {
                    {
                        let mut vec = Vec::with_capacity(#count);

                        #inner

                        vec
                    }.into()
                });
            } else if let Expr::Block(mut block_expr) = value.expr.clone() {
                self.visit_expr_block_mut(&mut block_expr);

                value.expr = Expr::MethodCall(parse_quote! { #block_expr.into() });
            } else if let Expr::Call(mut call_expr) = value.expr.clone() {
                self.visit_expr_call_mut(&mut call_expr);

                value.expr = Expr::MethodCall(parse_quote! { #call_expr.into() });
            } else if let Expr::Lit(mut lit_expr) = value.expr.clone() {
                self.visit_expr_lit_mut(&mut lit_expr);

                value.expr = Expr::MethodCall(parse_quote! { #lit_expr.into() });
            } else if let Expr::MethodCall(mut method_call) = value.expr.clone() {
                self.visit_expr_method_call_mut(&mut method_call);

                // Don't add `.into()` if there already is one
                if method_call.method != "into" {
                    value.expr = Expr::MethodCall(parse_quote! { #method_call.into() });
                }
            } else if let Expr::Paren(mut paren_expr) = value.expr.clone() {
                self.visit_expr_paren_mut(&mut paren_expr);

                value.expr = Expr::MethodCall(parse_quote! { #paren_expr.into() });
            } else if let Expr::Reference(mut reference_expr) = value.expr.clone() {
                self.visit_expr_reference_mut(&mut reference_expr);

                value.expr = Expr::MethodCall(parse_quote! { (#reference_expr).into() });
            } else if let Expr::Struct(mut struct_expr) = value.expr.clone() {
                self.visit_expr_struct_mut(&mut struct_expr);

                value.expr = Expr::MethodCall(parse_quote! { #struct_expr.into() });
            } else if let Expr::Closure(mut closure_expr) = value.expr.clone() {
                // Wrap closures in parentheses so that it can `.into()` to Callbacks properly
                self.visit_expr_closure_mut(&mut closure_expr);

                value.expr = Expr::MethodCall(parse_quote! { (#closure_expr).into() });
            }
        }
    }
}

struct WidgetStruct {
    attrs: Vec<syn::Attribute>,
    qself: Option<syn::QSelf>,
    path: syn::Path,
    brace_token: syn::token::Brace,
    fields: Punctuated<WidgetFieldValue, Token![,]>,
    dot2_token: Option<Token![..]>,
    rest: Option<Box<Expr>>,
}

impl Parse for WidgetStruct {
    fn parse(input: ParseStream) -> syn::parse::Result<Self> {
        let lookahead = input.lookahead1();

        if lookahead.peek(Token![<]) {
            let _widget_start_token = input.parse::<Token![<]>()?;

            let path = input.parse::<TypePath>()?;

            let _widget_end_token = input.parse::<Token![>]>()?;

            let content;
            let brace_token = braced!(content in input);

            let mut fields = Punctuated::new();
            while !content.is_empty() {
                if content.peek(Token![..]) {
                    return Ok(WidgetStruct {
                        attrs: Vec::new(),
                        qself: path.qself,
                        path: path.path,
                        brace_token,
                        fields,
                        dot2_token: Some(content.parse()?),
                        rest: if content.is_empty() {
                            None
                        } else {
                            Some(Box::new(content.parse()?))
                        },
                    });
                }

                fields.push(content.parse()?);
                if content.is_empty() {
                    break;
                }
                let punct: Token![,] = content.parse()?;
                fields.push_punct(punct);
            }

            if !fields.is_empty() && !fields.trailing_punct() {
                fields.push_punct(Comma::default());
            }

            Ok(WidgetStruct {
                attrs: Vec::new(),
                qself: path.qself,
                path: path.path,
                brace_token,
                fields,
                dot2_token: None,
                rest: None,
            })
        } else {
            Err(lookahead.error())
        }
    }
}

impl ToTokens for WidgetStruct {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        // Surround the struct with a call to `agui_core::widget::IntoWidget::into_widget(this)`

        resolve_agui_path().to_tokens(tokens);
        Token![::](Span::call_site()).to_tokens(tokens);
        Ident::new("widget", Span::call_site()).to_tokens(tokens);
        Token![::](Span::call_site()).to_tokens(tokens);
        Ident::new("IntoWidget", Span::call_site()).to_tokens(tokens);
        Token![::](Span::call_site()).to_tokens(tokens);
        Ident::new("into_widget", Span::call_site()).to_tokens(tokens);

        Paren::default().surround(tokens, |tokens| {
            for attr in &self.attrs {
                attr.to_tokens(tokens);
            }

            if let Some(ref qself) = self.qself {
                qself.lt_token.to_tokens(tokens);
                qself.ty.to_tokens(tokens);

                let pos = cmp::min(qself.position, self.path.segments.len());
                let mut segments = self.path.segments.pairs();

                if pos > 0 {
                    match &qself.as_token {
                        Some(t) => t.to_tokens(tokens),
                        None => syn::token::As::default().to_tokens(tokens),
                    }

                    self.path.leading_colon.to_tokens(tokens);
                    for (i, segment) in segments.by_ref().take(pos).enumerate() {
                        if i + 1 == pos {
                            segment.value().to_tokens(tokens);
                            qself.gt_token.to_tokens(tokens);
                            segment.punct().to_tokens(tokens);
                        } else {
                            segment.to_tokens(tokens);
                        }
                    }
                } else {
                    qself.gt_token.to_tokens(tokens);
                    self.path.leading_colon.to_tokens(tokens);
                }

                for segment in segments {
                    segment.to_tokens(tokens);
                }
            } else {
                // // Add "Props" to the end of the path
                // let mut path = self.path.clone();

                // let last_segment = path.segments.last_mut().unwrap();

                // last_segment.ident = Ident::new(
                //     &format!("{}Props", last_segment.ident),
                //     last_segment.ident.span(),
                // );

                self.path.to_tokens(tokens);
            }

            // Call `::builder()` on the type
            Token![::](Span::call_site()).to_tokens(tokens);
            Ident::new("builder", Span::call_site()).to_tokens(tokens);
            Paren::default().surround(tokens, |_| {});

            // Call the builder with the fields
            for field in &self.fields {
                field.to_tokens(tokens);
            }

            // Call `::build()` on the builder
            Token![.](Span::call_site()).to_tokens(tokens);
            Ident::new("build", Span::call_site()).to_tokens(tokens);
            Paren::default().surround(tokens, |_| {});
        });
    }
}

struct WidgetFieldValue {
    attrs: Vec<syn::Attribute>,
    member: syn::Ident,

    /// The colon in `Struct { x: x }`. If written in shorthand like
    /// `Struct { x }`, there is no colon.
    colon_token: Option<Token![:]>,

    expr: WidgetFieldExpr,
}

impl Parse for WidgetFieldValue {
    fn parse(input: ParseStream) -> syn::parse::Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;

        let member: Member = input.parse()?;

        if let Member::Named(ident) = &member {
            if input.peek(Token![:]) {
                return Ok(Self {
                    attrs,
                    member: ident.clone(),
                    colon_token: Some(input.parse()?),
                    expr: input.parse()?,
                });
            } else {
                return Ok(Self {
                    attrs,
                    member: ident.clone(),
                    colon_token: None,
                    expr: WidgetFieldExpr::Expr(Expr::Path(ExprPath {
                        attrs: Vec::new(),
                        qself: None,
                        path: Path::from(ident.clone()),
                    })),
                });
            }
        }

        Err(input.error("expected named field"))
    }
}

impl ToTokens for WidgetFieldValue {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        for attr in &self.attrs {
            attr.to_tokens(tokens);
        }

        // Turn the field into a function call
        Token![.](Span::call_site()).to_tokens(tokens);
        self.member.to_tokens(tokens);
        Paren::default().surround(tokens, |tokens| {
            self.expr.to_tokens(tokens);
        });
    }
}

enum WidgetFieldExpr {
    Widget(WidgetStruct),
    Expr(Expr),
}

impl Parse for WidgetFieldExpr {
    fn parse(input: ParseStream) -> syn::parse::Result<Self> {
        let lookahead = input.lookahead1();

        if lookahead.peek(Token![<]) {
            Ok(Self::Widget(input.parse()?))
        } else {
            Ok(Self::Expr(input.parse()?))
        }
    }
}

impl ToTokens for WidgetFieldExpr {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            Self::Widget(widget) => {
                // Ident::new("Into", Span::call_site()).to_tokens(tokens);
                // Token![::](Span::call_site()).to_tokens(tokens);
                // Ident::new("into", Span::call_site()).to_tokens(tokens);

                // Paren::default().surround(tokens, |tokens| widget.to_tokens(tokens));

                widget.to_tokens(tokens)
            }
            Self::Expr(expr) => expr.to_tokens(tokens),
        }
    }
}

pub(crate) fn build_impl(item: TokenStream2) -> TokenStream2 {
    let expr: WidgetStruct = match parse2(item) {
        Ok(item) => item,
        Err(err) => return err.into_compile_error(),
    };

    parse_quote! {
        {
            #[allow(clippy::needless_update)]
            {
                #expr
            }
        }
    }

    // let mut expr = match parse2(item) {
    //     Ok(item) => item,
    //     Err(err) => return err.into_compile_error(),
    // };

    // let mut visitor = BuildVisitor {};

    // if let Expr::Array(mut array_expr) = expr {
    //     visitor.visit_expr_array_mut(&mut array_expr);

    //     let count = array_expr.elems.len();

    //     let mut inner = quote::quote! {};

    //     for expr in array_expr.elems {
    //         inner.extend(quote::quote! {
    //             vec.push(#expr.into());
    //         });
    //     }

    //     expr = Expr::Block(parse_quote! {
    //         {
    //             let mut vec: Vec<#agui_core::widget::WidgetRef> = Vec::with_capacity(#count);

    //             #inner

    //             vec
    //         }
    //     });
    // } else {
    //     visitor.visit_expr_mut(&mut expr);
    // }

    // let stream = expr.into_token_stream();

    // parse_quote! {
    //     #[allow(clippy::needless_update)]
    //     {
    //         (#stream).into()
    //     }
    // }
}
