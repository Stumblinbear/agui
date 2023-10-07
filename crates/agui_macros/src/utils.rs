use std::path::PathBuf;

use proc_macro2::Span;
use syn::{Ident, Path, PathSegment};
use toml::{map::Map, Value};

pub fn resolve_package_path(pkg_name: &'static str) -> Path {
    let crate_name = std::env::var("CARGO_PKG_NAME").unwrap();

    if crate_name == pkg_name {
        return syn::parse_quote!(crate);
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
            .map(|deps| deps.as_table().unwrap().keys().any(|name| name == pkg_name))
            .or_else(|| {
                manifest
                    .get("dependencies")
                    .map(|deps| deps.as_table().unwrap().keys().any(|name| name == pkg_name))
            })
            .unwrap_or_default();

        if has_core_dep {
            return Path::from(PathSegment::from(Ident::new(pkg_name, Span::call_site())));
        }
    }

    syn::parse_quote!(agui)
}
