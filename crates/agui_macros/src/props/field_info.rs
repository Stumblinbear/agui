use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    parse::{Error, Parser},
    spanned::Spanned,
};

use super::util::{
    expr_to_single_string, ident_to_type, path_to_single_string, strip_raw_ident_prefix,
};

#[derive(Debug)]
pub struct FieldInfo<'a> {
    pub ordinal: usize,
    pub name: &'a syn::Ident,
    pub generic_ident: syn::Ident,
    pub ty: &'a syn::Type,
    pub builder_attr: FieldBuilderAttr<'a>,
}

impl<'a> FieldInfo<'a> {
    pub fn new(
        ordinal: usize,
        field: &'a syn::Field,
        field_defaults: FieldBuilderAttr<'a>,
    ) -> Result<FieldInfo<'a>, Error> {
        if let Some(ref name) = field.ident {
            Ok(FieldInfo {
                ordinal,
                name,
                generic_ident: syn::Ident::new(
                    &format!("__{}", strip_raw_ident_prefix(name.to_string())),
                    Span::call_site(),
                ),
                ty: &field.ty,
                builder_attr: field_defaults.with(&field.attrs)?,
            })
        } else {
            Err(Error::new(field.span(), "Nameless field in struct"))
        }
    }

    pub fn generic_ty_param(&self) -> syn::GenericParam {
        syn::GenericParam::Type(self.generic_ident.clone().into())
    }

    pub fn type_ident(&self) -> syn::Type {
        ident_to_type(self.generic_ident.clone())
    }

    pub fn tuplized_type_ty_param(&self) -> syn::Type {
        let mut types = syn::punctuated::Punctuated::default();
        types.push(self.ty.clone());
        types.push_punct(Default::default());
        syn::TypeTuple {
            paren_token: Default::default(),
            elems: types,
        }
        .into()
    }

    pub fn type_from_inside_option(&self) -> Option<&syn::Type> {
        let path = if let syn::Type::Path(type_path) = self.ty {
            if type_path.qself.is_some() {
                return None;
            }
            &type_path.path
        } else {
            return None;
        };
        let segment = path.segments.last()?;
        if segment.ident != "Option" {
            return None;
        }
        let generic_params =
            if let syn::PathArguments::AngleBracketed(generic_params) = &segment.arguments {
                generic_params
            } else {
                return None;
            };
        if let syn::GenericArgument::Type(ty) = generic_params.args.first()? {
            Some(ty)
        } else {
            None
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct FieldBuilderAttr<'a> {
    pub doc: Option<syn::Expr>,

    pub deprecated: Option<&'a syn::Attribute>,
    pub default: Option<syn::Expr>,
    pub skip: Option<Span>,
    pub auto_into: Option<Span>,
    // It's unclear if stripping options is a good idea for widgets, since many will likely want to be
    // able to conditionally set the value to None. For now, this is disabled.
    // pub strip_option: Option<Span>,
}

impl<'a> FieldBuilderAttr<'a> {
    pub fn with(mut self, attrs: &'a [syn::Attribute]) -> Result<Self, Error> {
        for attr in attrs {
            let list = match &attr.meta {
                syn::Meta::List(list) => {
                    let Some(path) = path_to_single_string(&list.path) else {
                        continue;
                    };

                    if path == "deprecated" {
                        self.deprecated = Some(attr);
                        continue;
                    }

                    if path != "prop" {
                        continue;
                    }

                    list
                }

                syn::Meta::Path(path) | syn::Meta::NameValue(syn::MetaNameValue { path, .. }) => {
                    if path_to_single_string(path).as_deref() == Some("deprecated") {
                        self.deprecated = Some(attr);
                    };

                    continue;
                }
            };

            if list.tokens.is_empty() {
                return Err(syn::Error::new_spanned(list, "Expected prop(…)"));
            }

            let parser = syn::punctuated::Punctuated::<_, syn::token::Comma>::parse_terminated;
            let exprs = parser.parse2(list.tokens.clone())?;
            for expr in exprs {
                self.apply_meta(expr)?;
            }
        }

        Ok(self)
    }

    pub fn apply_meta(&mut self, expr: syn::Expr) -> Result<(), Error> {
        match expr {
            syn::Expr::Assign(assign) => {
                let name = expr_to_single_string(&assign.left)
                    .ok_or_else(|| Error::new_spanned(&assign.left, "Expected identifier"))?;
                match name.as_str() {
                    "default" => {
                        self.default = Some(*assign.right);
                        Ok(())
                    }
                    "default_code" => {
                        if let syn::Expr::Lit(syn::ExprLit {
                            lit: syn::Lit::Str(code),
                            ..
                        }) = *assign.right
                        {
                            use std::str::FromStr;
                            let tokenized_code = TokenStream::from_str(&code.value())?;
                            self.default = Some(
                                syn::parse2(tokenized_code)
                                    .map_err(|e| Error::new_spanned(code, format!("{}", e)))?,
                            );
                        } else {
                            return Err(Error::new_spanned(assign.right, "Expected string"));
                        }
                        Ok(())
                    }
                    "doc" => {
                        self.doc = Some(*assign.right);
                        Ok(())
                    }
                    _ => Err(Error::new_spanned(
                        &assign,
                        format!("Unknown parameter {:?}", name),
                    )),
                }
            }

            syn::Expr::Path(path) => {
                let name = path_to_single_string(&path.path)
                    .ok_or_else(|| Error::new_spanned(&path, "Expected identifier"))?;

                macro_rules! handle_fields {
                    ( $( $flag:expr, $field:ident, $already:expr, $setter:expr; )* ) => {
                        match name.as_str() {
                            $(
                                $flag => {
                                    if self.$field.is_some() {
                                        Err(Error::new(path.span(), concat!("Illegal setting - field is already ", $already)))
                                    } else {
                                        self.$field = Some($setter);
                                        Ok(())
                                    }
                                }
                            )*
                            _ => Err(Error::new_spanned(
                                    &path,
                                    format!("Unknown setter parameter {:?}", name),
                            ))
                        }
                    }
                }

                handle_fields!(
                    "skip", skip, "skipped", path.span();
                    "into", auto_into, "calling into() on the argument", path.span();
                    // "strip_option", strip_option, "putting the argument in Some(...)", path.span();
                    "default", default, "defaulted", syn::parse2(quote!(::core::default::Default::default())).unwrap();
                )
            }

            syn::Expr::Unary(syn::ExprUnary {
                op: syn::UnOp::Not(_),
                expr,
                ..
            }) => {
                if let syn::Expr::Path(path) = *expr {
                    let name = path_to_single_string(&path.path)
                        .ok_or_else(|| Error::new_spanned(&path, "Expected identifier"))?;

                    match name.as_str() {
                        "default" => {
                            self.default = None;
                            Ok(())
                        }

                        "skip" => {
                            self.skip = None;
                            Ok(())
                        }

                        "auto_into" => {
                            self.auto_into = None;
                            Ok(())
                        }

                        // "strip_option" => {
                        //     self.strip_option = None;
                        //     Ok(())
                        // }
                        _ => Err(Error::new_spanned(path, "Unknown setting".to_owned())),
                    }
                } else {
                    Err(Error::new_spanned(
                        expr,
                        "Expected simple identifier".to_owned(),
                    ))
                }
            }
            _ => Err(Error::new_spanned(expr, "Expected (<...>=<...>)")),
        }
    }
}
