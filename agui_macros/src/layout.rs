use quote::quote;
use darling::{FromMeta, ToTokens};

#[derive(Debug, Clone, Copy, FromMeta)]
#[darling(default)]
pub enum LayoutType {
    Row, Column, None
}

impl Default for LayoutType {
    fn default() -> Self {
        Self::Column
    }
}

impl ToTokens for LayoutType {
    fn to_tokens(&self, tokens: &mut quote::__private::TokenStream) {
        #[cfg(feature = "internal")]
        let agui_core = quote! { agui_core };
        #[cfg(not(feature = "internal"))]
        let agui_core = quote! { agui };
    
        tokens.extend(match self {
            LayoutType::Row => quote!{ #agui_core::unit::LayoutType::Row },
            LayoutType::Column => quote!{ #agui_core::unit::LayoutType::Column },
            LayoutType::None => unreachable!(),
        });
    }
}