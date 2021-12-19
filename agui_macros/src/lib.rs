extern crate proc_macro;

use darling::{FromDeriveInput, FromMeta};
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, AttributeArgs, Ident, ItemFn};

mod layout;

use layout::LayoutType;

#[derive(Debug, FromMeta)]
struct MacroArgs {
    #[darling(default)]
    layout: Option<Ident>,
}

#[proc_macro_attribute]
pub fn widget(args: TokenStream, input: TokenStream) -> TokenStream {
    let attr_args = parse_macro_input!(args as AttributeArgs);
    let _input = parse_macro_input!(input as ItemFn);

    let _args = match MacroArgs::from_list(&attr_args) {
        Ok(v) => v,
        Err(e) => {
            return TokenStream::from(e.write_errors());
        }
    };

    todo!()
}

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(widget))]
struct WidgetDeriveInput {
    #[darling(default)]
    layout: LayoutType,

    ident: syn::Ident,
    generics: syn::Generics,
}

#[proc_macro_derive(Widget, attributes(widget))]
pub fn widget_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).expect("Couldn't parse item");

    let args = WidgetDeriveInput::from_derive_input(&ast).unwrap();

    #[cfg(feature = "internal")]
    let agui_core = quote! { agui_core };
    #[cfg(not(feature = "internal"))]
    let agui_core = quote! { agui };

    let ident = args.ident;
    let (impl_generics, ty_generics, where_clause) = args.generics.split_for_impl();

    let widget_type_impl = quote! {
        impl #impl_generics #agui_core::widget::WidgetType for #ident #ty_generics #where_clause {
            fn get_type_id(&self) -> std::any::TypeId {
                std::any::TypeId::of::<Self>()
            }
        }
    };

    let layout_type = args.layout;

    // We prevent the generation of the layout implementation if they've specified #[widget(layout = "none")]
    let widget_layout_impl = match layout_type {
        LayoutType::None => quote! {},
        _ => quote! {
            impl #impl_generics #agui_core::widget::WidgetLayout for #ident #ty_generics #where_clause {
                fn layout_type(&self) -> #agui_core::widget::LayoutType {
                    #layout_type
                }
            }
        },
    };

    TokenStream::from(quote! {
        #widget_type_impl

        #widget_layout_impl

        impl #impl_generics #agui_core::widget::Widget for #ident #ty_generics #where_clause { }
    })
}
