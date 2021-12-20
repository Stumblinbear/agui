use darling::FromDeriveInput;
use proc_macro::TokenStream;
use quote::quote;

use crate::layout::LayoutType;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(widget))]
struct WidgetDeriveInput {
    #[darling(default)]
    layout: LayoutType,
    
    #[darling(default)]
    into: Option<bool>,

    ident: syn::Ident,
    generics: syn::Generics,
}

pub fn parse_widget_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).expect("Couldn't parse item");

    let args = WidgetDeriveInput::from_derive_input(&ast).unwrap();

    #[cfg(feature = "internal")]
    let agui_core = quote! { agui_core };
    #[cfg(not(feature = "internal"))]
    let agui_core = quote! { agui };

    let ident = args.ident;
    let (impl_generics, ty_generics, where_clause) = args.generics.split_for_impl();

    let widget_type_impl = quote! {
        impl #impl_generics #agui_core::WidgetType for #ident #ty_generics #where_clause {
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
            impl #impl_generics #agui_core::WidgetLayout for #ident #ty_generics #where_clause {
                fn layout_type(&self) -> #agui_core::unit::LayoutType {
                    #layout_type
                }
            }
        },
    };

    let widget_ref_impl = if args.into.unwrap_or(true) {
        quote! {
            impl #impl_generics From<#ident #ty_generics> for #agui_core::WidgetRef #where_clause {
                fn from(widget: #ident #ty_generics) -> Self {
                    Self::new(widget)
                }
            }

            impl #impl_generics From<#ident #ty_generics> for Option<#agui_core::WidgetRef> #where_clause {
                fn from(widget: #ident #ty_generics) -> Self {
                    Some(#agui_core::WidgetRef::new(widget))
                }
            }
        }
    }else{
        quote! { }
    };

    TokenStream::from(quote! {
        #widget_type_impl

        #widget_layout_impl

        impl #impl_generics #agui_core::Widget for #ident #ty_generics #where_clause { }

        #widget_ref_impl
    })
}