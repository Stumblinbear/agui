use std::path::PathBuf;

use proc_macro2::TokenStream as TokenStream2;
use toml::{map::Map, Value};

static AGUI_CORE: &str = "agui_core";

pub fn resolve_agui_path() -> TokenStream2 {
    let crate_name = std::env::var("CARGO_PKG_NAME").unwrap();

    if crate_name == "agui_core" {
        return quote::quote! { crate };
    }

    let manifest: Option<Map<String, Value>> = std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .map(|mut path| {
            path.push("Cargo.toml");
            let manifest = std::fs::read_to_string(path).unwrap();
            toml::from_str(&manifest).unwrap()
        });

    if let Some(manifest) = manifest {
        let has_core_dep = manifest
            .get("dependencies")
            .map(|deps| {
                deps.as_table()
                    .unwrap()
                    .keys()
                    .any(|pkg_name| pkg_name == AGUI_CORE)
            })
            .or_else(|| {
                manifest.get("dependencies").map(|deps| {
                    deps.as_table()
                        .unwrap()
                        .keys()
                        .any(|pkg_name| pkg_name == AGUI_CORE)
                })
            })
            .unwrap_or_default();

        if has_core_dep {
            return quote::quote! { agui_core };
        }
    }

    quote::quote! { agui }
}
