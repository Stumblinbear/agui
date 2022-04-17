use core::panic;

use heck::ToUpperCamelCase;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2, TokenTree};
use syn::{
    parse2, parse_quote,
    punctuated::Punctuated,
    token::{Gt, Lt},
    visit_mut::{visit_item_fn_mut, VisitMut},
    AngleBracketedGenericArguments, GenericArgument, ItemFn, Pat, PatIdent, Path, PathArguments,
    PathSegment, ReturnType, Type, TypePath,
};

#[derive(Default)]
struct FunctionVisitor {
    fn_ident: Option<Ident>,

    ident: Option<String>,
    args: Vec<(PatIdent, Type)>,
}

impl FunctionVisitor {}

impl VisitMut for FunctionVisitor {
    fn visit_item_fn_mut(&mut self, func: &mut ItemFn) {
        visit_item_fn_mut(self, func);

        if func.sig.variadic.is_some() {
            panic!("functional widgets do not support variadic arguments");
        }

        if let ReturnType::Default = func.sig.output {
            panic!("return type must be BuildResult");
        }

        if let ReturnType::Type(_, ty) = &func.sig.output {
            if !matches!(**ty, Type::Path(_)) {
                panic!("return type must be BuildResult");
            }
        }

        self.fn_ident = Some(func.sig.ident.clone());
        self.ident = Some(func.sig.ident.to_string().to_upper_camel_case());

        for input in &mut func.sig.inputs {
            match input {
                syn::FnArg::Receiver(_) => {
                    panic!("functional widgets do not support self");
                }
                syn::FnArg::Typed(arg) => {
                    if self.args.is_empty() {
                        if let Type::Reference(ty) = arg.ty.as_mut() {
                            if ty.mutability.is_none() {
                                panic!("first argument must be &mut BuildContext");
                            }

                            if let Type::Path(ty_path) = ty.elem.as_mut() {
                                let segment = ty_path.path.segments.last_mut().unwrap();

                                if segment.ident != "BuildContext" || !segment.arguments.is_empty()
                                {
                                    panic!("first argument must be &mut BuildContext");
                                }

                                let mut self_path = Punctuated::default();

                                self_path.push_value(PathSegment {
                                    ident: Ident::new(
                                        self.ident.as_ref().unwrap(),
                                        Span::call_site(),
                                    ),
                                    arguments: PathArguments::None,
                                });

                                let mut args = Punctuated::default();

                                args.push_value(GenericArgument::Type(Type::Path(TypePath {
                                    qself: None,
                                    path: Path {
                                        leading_colon: None,
                                        segments: self_path,
                                    },
                                })));

                                segment.arguments =
                                    PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                                        colon2_token: None,
                                        lt_token: Lt::default(),
                                        args,
                                        gt_token: Gt::default(),
                                    });
                            } else {
                                panic!("first argument must be &mut BuildContext");
                            }
                        }
                    }

                    let pat = &*arg.pat;
                    let ty = &*arg.ty;

                    if let Pat::Ident(ident) = pat {
                        self.args.push((ident.clone(), ty.clone()));
                    } else {
                        panic!("unexpected argument: {:?}", pat);
                    }
                }
            }
        }

        if self.args.is_empty() {
            panic!("first argument must be &mut BuildContext");
        }
    }
}

pub(crate) fn parse_functional_widget(args: TokenStream2, item: TokenStream2) -> TokenStream2 {
    let state: Option<TokenTree> = match parse2(args) {
        Ok(item) => item,
        Err(err) => return err.into_compile_error(),
    };

    let state = match state {
        Some(state) => quote::quote! { #state },
        None => quote::quote! { () },
    };

    let mut item = match parse2(item) {
        Ok(item) => item,
        Err(err) => return err.into_compile_error(),
    };

    let mut visitor = FunctionVisitor::default();

    visitor.visit_item_fn_mut(&mut item);

    let fn_ident = visitor
        .fn_ident
        .expect("functional widget formatted incorrectly");

    let ident = Ident::new(
        &visitor
            .ident
            .expect("functional widget formatted incorrectly"),
        Span::call_site(),
    );

    let mut fields = quote::quote! {};
    let mut args = quote::quote! { ctx };

    visitor.args.remove(0);

    for (ident, ty) in &visitor.args {
        fields.extend(quote::quote! {
            pub #ident: #ty,
        });

        args.extend(quote::quote! {
            , self.#ident.clone()
        });
    }

    // #[cfg(feature = "internal")]
    // let agui_core = quote::quote! { agui_core };
    // #[cfg(not(feature = "internal"))]
    let agui_core = quote::quote! { agui };

    parse_quote! {
        #item

        #[derive(Debug, Default)]
        pub struct #ident {
            #fields
        }

        impl #agui_core::widget::StatefulWidget for #ident {
            type State = #state;

            fn build(&self, ctx: &mut #agui_core::widget::BuildContext<Self>) -> #agui_core::widget::BuildResult {
                #fn_ident(#args)
            }
        }
    }
}
