use proc_macro2::TokenStream as TokenStream2;
use syn::{parse2, parse_quote, Field, ItemStruct};

use crate::utils::resolve_agui_path;

pub fn impl_paint_widget(input: TokenStream2) -> TokenStream2 {
    let agui_core = resolve_agui_path();

    let item: ItemStruct = match parse2(input) {
        Ok(item) => item,
        Err(err) => return err.into_compile_error(),
    };

    let ident = &item.ident;
    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

    // Find a field with the #[child] attribute.
    let child_field = item
        .fields
        .iter()
        .find(|field| field.attrs.iter().any(|attr| attr.path.is_ident("child")));

    let build_impl: TokenStream2 = match child_field {
        Some(Field {
            ident: child_field,
            ty: child_ty,
            ..
        }) => {
            parse_quote! {
                impl #impl_generics #agui_core::widget::WidgetChild for #ident #ty_generics #where_clause {
                    type Child = #child_ty;

                    fn get_child(&self) -> Self::Child {
                        self.#child_field.clone()
                    }
                }
            }
        }

        None => parse_quote! {
            impl #impl_generics #agui_core::widget::WidgetChild for #ident #ty_generics #where_clause {
                type Child = ();

                fn get_child(&self) -> Self::Child {}
            }
        },
    };

    parse_quote! {
        #build_impl

        impl #impl_generics #agui_core::widget::ElementBuilder for #ident #ty_generics #where_clause {
            fn create_element(self: std::rc::Rc<Self>) -> Box<dyn #agui_core::widget::element::WidgetElement>
            where
                Self: Sized
            {
                Box::new(#agui_core::widget::PaintElement::new(self))
            }
        }
    }
}
