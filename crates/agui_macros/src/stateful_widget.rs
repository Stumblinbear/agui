use proc_macro2::TokenStream as TokenStream2;
use syn::{parse2, parse_quote, ItemStruct};

use crate::{props::impl_props_derive, utils::resolve_package_path};

pub fn impl_stateful_widget(input: TokenStream2) -> TokenStream2 {
    let agui_core = resolve_package_path("agui_core");
    let agui_elements = resolve_package_path("agui_elements");

    let item: ItemStruct = match parse2(input) {
        Ok(item) => item,
        Err(err) => return err.into_compile_error(),
    };

    let ident = &item.ident;
    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

    let props_impl = impl_props_derive(&item).unwrap_or_else(|err| err.into_compile_error());

    parse_quote! {
        #props_impl

        impl #impl_generics #agui_core::element::ElementBuilder for #ident #ty_generics #where_clause {
            fn create_element(self: std::rc::Rc<Self>) -> #agui_core::element::ElementType
            where
                Self: Sized
            {
                #agui_core::element::ElementType::new_widget(#agui_elements::stateful::StatefulWidgetElement::new(self))
            }
        }
    }
}
