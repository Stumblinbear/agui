// Yoink: idanarye/rust-typed-builder
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse::Error, spanned::Spanned};
use syn::{parse2, ItemStruct};

mod field_info;
mod struct_info;
mod util;

pub fn impl_widget_props(input: TokenStream2) -> TokenStream2 {
    let item: ItemStruct = match parse2(input) {
        Ok(item) => item,
        Err(err) => return err.into_compile_error(),
    };

    impl_props_derive(&item).unwrap_or_else(|err| err.into_compile_error())
}

pub fn impl_props_derive(ast: &syn::ItemStruct) -> Result<TokenStream2, Error> {
    let data = match &ast.fields {
        syn::Fields::Named(fields) => {
            let struct_info = struct_info::StructInfo::new(ast, fields.named.iter())?;
            let builder_creation = struct_info.builder_creation_impl()?;
            let fields = struct_info
                .included_fields()
                .map(|f| struct_info.field_impl(f))
                .collect::<Result<TokenStream2, _>>()?;
            let required_fields = struct_info
                .included_fields()
                .filter(|f| f.builder_attr.default.is_none())
                .map(|f| struct_info.required_field_impl(f));
            let build_method = struct_info.build_method_impl();

            quote! {
                #builder_creation
                #fields
                #(#required_fields)*
                #build_method
            }
        }

        syn::Fields::Unnamed(_) => {
            return Err(Error::new(
                ast.span(),
                "TypedBuilder is not supported for tuple structs",
            ))
        }

        syn::Fields::Unit => {
            let fields = Vec::new();
            let struct_info = struct_info::StructInfo::new(ast, fields.iter())?;
            let builder_creation = struct_info.builder_creation_impl()?;
            let fields = struct_info
                .included_fields()
                .map(|f| struct_info.field_impl(f))
                .collect::<Result<TokenStream2, _>>()?;
            let required_fields = struct_info
                .included_fields()
                .filter(|f| f.builder_attr.default.is_none())
                .map(|f| struct_info.required_field_impl(f));
            let build_method = struct_info.build_method_impl();

            quote! {
                #builder_creation
                #fields
                #(#required_fields)*
                #build_method
            }
        }
    };

    Ok(data)
}
