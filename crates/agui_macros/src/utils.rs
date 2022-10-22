use proc_macro2::TokenStream as TokenStream2;

pub fn resolve_agui_path() -> TokenStream2 {
    #[cfg(feature = "internal")]
    {
        let crate_name = std::env::var("CARGO_PKG_NAME").unwrap();

        if crate_name == "agui_core" {
            quote::quote! { crate }
        } else {
            quote::quote! { agui_core }
        }
    }

    #[cfg(not(feature = "internal"))]
    quote::quote! { agui };
}
