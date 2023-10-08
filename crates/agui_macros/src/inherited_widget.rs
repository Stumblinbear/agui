use proc_macro2::TokenStream as TokenStream2;
use syn::{parse2, parse_quote, ItemStruct};

use crate::{props::impl_props_derive, utils::resolve_package_path};

pub fn impl_inherited_widget(input: TokenStream2) -> TokenStream2 {
    let agui_core = resolve_package_path("agui_core");
    let agui_inheritance = resolve_package_path("agui_inheritance");

    let item: ItemStruct = match parse2(input) {
        Ok(item) => item,
        Err(err) => return err.into_compile_error(),
    };

    let ident = &item.ident;
    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

    let props_impl = impl_props_derive(&item).unwrap_or_else(|err| err.into_compile_error());

    parse_quote! {
        #props_impl

        impl #impl_generics #agui_core::widget::IntoWidget for #ident #ty_generics #where_clause {
            fn into_widget(self) -> #agui_core::widget::Widget {
                #agui_core::widget::Widget::new(self)
            }
        }

        impl #impl_generics #agui_core::element::ElementBuilder for #ident #ty_generics #where_clause {
            fn create_element(self: std::rc::Rc<Self>) -> #agui_core::element::ElementType
            where
                Self: Sized
            {
                #agui_core::element::ElementType::Widget(Box::new(#agui_inheritance::InheritedElement::new(self)))
            }
        }
    }
}
