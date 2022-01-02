use heck::ToUpperCamelCase;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{
    parse2, parse_quote,
    visit::{visit_item_fn, Visit},
    ItemFn, Pat, PatIdent, ReturnType, Type,
};

#[derive(Default)]
struct FunctionVisitor {
    fn_ident: Option<Ident>,

    ident: Option<String>,
    args: Vec<(PatIdent, Type)>,
}

impl FunctionVisitor {}

impl Visit<'_> for FunctionVisitor {
    fn visit_item_fn(&mut self, func: &'_ ItemFn) {
        visit_item_fn(self, func);

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

        for input in &func.sig.inputs {
            match input {
                syn::FnArg::Receiver(_) => {
                    panic!("functional widgets do not support self");
                }
                syn::FnArg::Typed(arg) => {
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

        if !self.args.is_empty() {
            let (_, ty) = self.args.remove(0);

            if let Type::Reference(ty) = ty {
                if ty.mutability.is_some() {
                    panic!("first argument must be &WidgetContext");
                }

                if let Type::Path(ty_path) = &*ty.elem {
                    let segment = ty_path.path.segments.last().unwrap();

                    if segment.ident != "WidgetContext" {
                        panic!("first argument must be &WidgetContext");
                    }
                } else {
                    panic!("first argument must be &WidgetContext");
                }
            }
        } else {
            panic!("first argument must be &WidgetContext");
        }
    }
}

pub(crate) fn parse_functional_widget(args: TokenStream2, item: TokenStream2) -> TokenStream2 {
    let item = match parse2(item) {
        Ok(item) => item,
        Err(err) => return err.into_compile_error(),
    };

    let mut visitor = FunctionVisitor::default();

    visitor.visit_item_fn(&item);

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

    for (ident, ty) in &visitor.args {
        fields.extend(quote::quote! {
            pub #ident: #ty,
        });

        args.extend(quote::quote! {
            , self.#ident.clone()
        });
    }

    #[cfg(feature = "internal")]
    let agui_core = quote! { agui };
    #[cfg(not(feature = "internal"))]
    let agui_core = quote! { agui };

    parse_quote! {
        #item

        #[derive(Default)]
        pub struct #ident {
            #fields
        }

        impl #agui_core::widget::WidgetType for #ident {
            fn get_type_id(&self) -> std::any::TypeId {
                std::any::TypeId::of::<Self>()
            }

            fn get_type_name(&self) -> &'static str {
                std::any::type_name::<Self>()
            }
        }

        impl #agui_core::widget::WidgetBuilder for #ident {
            fn build(&self, ctx: &#agui_core::context::WidgetContext) -> #agui_core::widget::BuildResult {
                #fn_ident(#args)
            }
        }

        impl #agui_core::widget::Widget for #ident { }

        impl From<#ident> for #agui_core::widget::WidgetRef {
            fn from(widget: #ident) -> Self {
                Self::new(widget)
            }
        }

        impl From<#ident> for Option<#agui_core::widget::WidgetRef> {
            fn from(widget: #ident) -> Self {
                Some(#agui_core::widget::WidgetRef::new(widget))
            }
        }

        impl From<#ident> for #agui_core::widget::BuildResult {
            fn from(widget: #ident) -> Self {
                Self::One(widget.into())
            }
        }
    }
}
