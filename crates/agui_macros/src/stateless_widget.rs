use proc_macro2::TokenStream as TokenStream2;
use syn::{parse2, parse_quote, ItemStruct};

use crate::utils::resolve_agui_path;

pub fn impl_stateless_widget(input: TokenStream2) -> TokenStream2 {
    let agui_core = resolve_agui_path();

    let item: ItemStruct = match parse2(input) {
        Ok(item) => item,
        Err(err) => return err.into_compile_error(),
    };

    let ident = &item.ident;
    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

    parse_quote! {
        impl #impl_generics #agui_core::widget::Widget for #ident #ty_generics #where_clause {
            fn into_widget(self: std::rc::Rc<Self>) -> Box<dyn #agui_core::widget::dispatch::WidgetDispatch>
            where
                Self: Sized
            {
                Box::new(#agui_core::widget::instance::WidgetInstance::new(self))
            }
        }

        impl #impl_generics #agui_core::widget::WidgetState for #ident #ty_generics #where_clause {
            type State = ();

            fn create_state(&self) -> Self::State {}
        }
    }
}
