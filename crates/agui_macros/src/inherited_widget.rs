use proc_macro2::TokenStream as TokenStream2;
use syn::{parse2, parse_quote, Field, ItemStruct};

use crate::utils::resolve_agui_path;

pub fn impl_inherited_widget(input: TokenStream2) -> TokenStream2 {
    let agui_core = resolve_agui_path();

    let item: ItemStruct = match parse2(input) {
        Ok(item) => item,
        Err(err) => return err.into_compile_error(),
    };

    // Find the field with the #[child] attribute.
    let Some(Field {
        ident: child_field,
        ty: child_ty,
        ..
    }) = item
        .fields
        .iter()
        .find(|field| field.attrs.iter().any(|attr| attr.path().is_ident("child")))
    else {
        return syn::Error::new_spanned(
            &item,
            "InheritedWidget must have a field with a #[child] attribute.",
        )
        .into_compile_error();
    };

    let ident = &item.ident;
    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

    parse_quote! {
        impl #impl_generics #agui_core::widget::WidgetChild for #ident #ty_generics #where_clause {
            type Child = #child_ty;

            fn get_child(&self) -> Self::Child {
                self.#child_field.clone()
            }
        }

        impl #impl_generics #agui_core::widget::IntoWidget for #ident #ty_generics #where_clause {
            fn into_widget(self) -> #agui_core::widget::Widget {
                #agui_core::widget::Widget::new(self)
            }
        }

        impl #impl_generics Into<Option<#agui_core::widget::Widget>> for #ident #ty_generics #where_clause {
            fn into(self) -> Option<#agui_core::widget::Widget> {
                use #agui_core::widget::IntoWidget;

                Some(self.into_widget())
            }
        }

        impl #impl_generics #agui_core::widget::ElementBuilder for #ident #ty_generics #where_clause {
            fn create_element(self: std::rc::Rc<Self>) -> Box<dyn #agui_core::widget::element::WidgetElement>
            where
                Self: Sized
            {
                Box::new(#agui_core::widget::InheritedElement::new(self))
            }
        }
    }
}
