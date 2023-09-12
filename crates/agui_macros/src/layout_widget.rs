use proc_macro2::TokenStream as TokenStream2;
use syn::{parse2, parse_quote, ItemStruct};

use crate::{props::impl_props_derive, utils::resolve_agui_path};

pub fn impl_layout_widget(input: TokenStream2) -> TokenStream2 {
    let agui_core = resolve_agui_path();

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

        impl #impl_generics #agui_core::widget::ElementBuilder for #ident #ty_generics #where_clause {
            fn create_element(self: std::rc::Rc<Self>) -> Box<dyn #agui_core::widget::element::WidgetElement>
            where
                Self: Sized
            {
                Box::new(#agui_core::widget::LayoutElement::new(self))
            }
        }
    }
}
