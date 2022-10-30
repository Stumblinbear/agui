use proc_macro2::TokenStream as TokenStream2;
use syn::{parse2, parse_quote, ItemStruct};

use crate::utils::resolve_agui_path;

pub fn impl_stateful_widget(input: TokenStream2) -> TokenStream2 {
    let agui_core = resolve_agui_path();

    let item: ItemStruct = match parse2(input) {
        Ok(item) => item,
        Err(err) => return err.into_compile_error(),
    };

    let ident = &item.ident;
    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

    parse_quote! {
        impl #impl_generics #agui_core::widget::WidgetDerive for #ident #ty_generics #where_clause {
            fn get_type_name(&self) -> &str {
                std::any::type_name::<Self>()
            }

            fn is_equal(&self, other: &dyn #agui_core::widget::WidgetDerive) -> bool {
                other
                    .downcast_ref::<Self>()
                    .map_or(false, |a| self == a)
            }

            fn create_element(self: std::rc::Rc<Self>) -> #agui_core::element::ElementType
            where
                Self: Sized
            {
                #agui_core::element::ElementType::new_inherited(self)
            }
        }

        impl #impl_generics #agui_core::widget::Widget for #ident #ty_generics #where_clause { }

        impl #impl_generics #agui_core::widget::InheritedWidget for #ident #ty_generics #where_clause { }
    }
}
