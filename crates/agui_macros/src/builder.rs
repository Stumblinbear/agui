use proc_macro2::TokenStream as TokenStream2;
use quote::ToTokens;
use syn::{
    parse2, parse_quote,
    visit_mut::{visit_expr_struct_mut, VisitMut},
    Expr, ExprStruct, FieldValue,
};

use crate::utils::resolve_agui_path;

struct BuildVisitor {}

impl BuildVisitor {}

impl VisitMut for BuildVisitor {
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

pub(crate) fn build_impl(item: TokenStream2) -> TokenStream2 {
    let agui_core = resolve_agui_path();

    let mut expr = match parse2(item) {
        Ok(item) => item,
        Err(err) => return err.into_compile_error(),
    };

    let mut visitor = BuildVisitor {};

    if let Expr::Array(mut array_expr) = expr {
        visitor.visit_expr_array_mut(&mut array_expr);

        let count = array_expr.elems.len();

        let mut inner = quote::quote! {};

        for expr in array_expr.elems {
            inner.extend(quote::quote! {
                vec.push(#expr.into());
            });
        }

        expr = Expr::Block(parse_quote! {
            {
                let mut vec: Vec<#agui_core::widget::WidgetRef> = Vec::with_capacity(#count);

                #inner

                vec
            }
        });
    } else {
        visitor.visit_expr_mut(&mut expr);
    }

    let stream = expr.into_token_stream();

    parse_quote! {
        #[allow(clippy::needless_update)]
        {
            (#stream).into()
        }
    }
}
