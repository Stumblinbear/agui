use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use syn::parse::{Error, Parser};

use crate::utils::resolve_agui_path;

use super::{
    field_info::{FieldBuilderAttr, FieldInfo},
    util::{
        empty_type, empty_type_tuple, modify_types_generics_hack, path_to_single_string,
        strip_raw_ident_prefix, type_tuple,
    },
};

#[derive(Debug)]
pub struct StructInfo<'a> {
    pub vis: &'a syn::Visibility,
    pub name: &'a syn::Ident,
    pub generics: &'a syn::Generics,
    pub fields: Vec<FieldInfo<'a>>,

    pub builder_attr: TypeBuilderAttr<'a>,
    pub builder_name: syn::Ident,
}

impl<'a> StructInfo<'a> {
    pub fn included_fields(&self) -> impl Iterator<Item = &FieldInfo<'a>> {
        self.fields.iter().filter(|f| f.builder_attr.skip.is_none())
    }

    pub fn new(
        ast: &'a syn::ItemStruct,
        fields: impl Iterator<Item = &'a syn::Field>,
    ) -> Result<StructInfo<'a>, Error> {
        let builder_attr = TypeBuilderAttr::new(&ast.attrs)?;
        let builder_name = strip_raw_ident_prefix(format!("{}Props", ast.ident));

        Ok(StructInfo {
            vis: &ast.vis,
            name: &ast.ident,
            generics: &ast.generics,
            fields: fields
                .enumerate()
                .map(|(i, f)| FieldInfo::new(i, f, builder_attr.field_defaults.clone()))
                .collect::<Result<_, _>>()?,
            builder_attr,
            builder_name: syn::Ident::new(&builder_name, ast.ident.span()),
        })
    }

    pub fn builder_creation_impl(&self) -> Result<TokenStream, Error> {
        let StructInfo {
            vis,
            ref name,
            ref builder_name,
            ..
        } = *self;
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        let empties_tuple = type_tuple(self.included_fields().map(|_| empty_type()));
        let mut all_fields_param_type: syn::TypeParam =
            syn::Ident::new("TypedBuilderFields", builder_name.span()).into();
        let all_fields_param = syn::GenericParam::Type(all_fields_param_type.clone());
        all_fields_param_type.default = Some(syn::Type::Tuple(empties_tuple.clone()));
        let b_generics = {
            let mut generics = self.generics.clone();
            generics
                .params
                .push(syn::GenericParam::Type(all_fields_param_type));
            generics
        };
        let generics_with_empty = modify_types_generics_hack(&ty_generics, |args| {
            args.push(syn::GenericArgument::Type(empties_tuple.clone().into()));
        });
        let phantom_generics = self.generics.params.iter().filter_map(|param| match param {
            syn::GenericParam::Lifetime(lifetime) => {
                let lifetime = &lifetime.lifetime;
                Some(quote!(&#lifetime ()))
            }
            syn::GenericParam::Type(ty) => {
                let ty = &ty.ident;
                Some(ty.to_token_stream())
            }
            syn::GenericParam::Const(_cnst) => None,
        });

        let (b_generics_impl, b_generics_ty, b_generics_where_extras_predicates) =
            b_generics.split_for_impl();
        let mut b_generics_where: syn::WhereClause = syn::parse2(quote! {
            where TypedBuilderFields: Clone
        })?;

        if let Some(predicates) = b_generics_where_extras_predicates {
            b_generics_where
                .predicates
                .extend(predicates.predicates.clone());
        }

        Ok(quote! {
            impl #impl_generics #name #ty_generics #where_clause {
                #[allow(dead_code, clippy::default_trait_access)]
                #vis fn builder() -> #builder_name #generics_with_empty {
                    #builder_name {
                        fields: #empties_tuple,
                        phantom: ::core::default::Default::default(),
                    }
                }
            }

            #[must_use]
            #[doc(hidden)]
            #[allow(dead_code, non_camel_case_types, non_snake_case)]
            #vis struct #builder_name #b_generics {
                fields: #all_fields_param,
                phantom: ::core::marker::PhantomData<(#( #phantom_generics ),*)>,
            }

            impl #b_generics_impl Clone for #builder_name #b_generics_ty #b_generics_where {
                #[allow(clippy::default_trait_access)]
                fn clone(&self) -> Self {
                    Self {
                        fields: self.fields.clone(),
                        phantom: ::core::marker::PhantomData,
                    }
                }
            }
        })
    }

    pub fn field_impl(&self, field: &FieldInfo) -> Result<TokenStream, Error> {
        let StructInfo {
            ref builder_name, ..
        } = *self;

        let descructuring = self.included_fields().map(|f| {
            if f.ordinal == field.ordinal {
                quote!(())
            } else {
                let name = f.name;
                name.to_token_stream()
            }
        });
        let reconstructing = self.included_fields().map(|f| f.name);

        let &FieldInfo {
            name: field_name,
            ty: field_type,
            ..
        } = field;
        let mut ty_generics: Vec<syn::GenericArgument> = self
            .generics
            .params
            .iter()
            .map(|generic_param| match generic_param {
                syn::GenericParam::Type(type_param) => {
                    let ident = type_param.ident.to_token_stream();
                    syn::parse2(ident).unwrap()
                }
                syn::GenericParam::Lifetime(lifetime_def) => {
                    syn::GenericArgument::Lifetime(lifetime_def.lifetime.clone())
                }
                syn::GenericParam::Const(const_param) => {
                    let ident = const_param.ident.to_token_stream();
                    syn::parse2(ident).unwrap()
                }
            })
            .collect();
        let mut target_generics_tuple = empty_type_tuple();
        let mut ty_generics_tuple = empty_type_tuple();
        let generics = {
            let mut generics = self.generics.clone();
            for f in self.included_fields() {
                if f.ordinal == field.ordinal {
                    ty_generics_tuple.elems.push_value(empty_type());
                    target_generics_tuple
                        .elems
                        .push_value(f.tuplized_type_ty_param());
                } else {
                    generics.params.push(f.generic_ty_param());
                    let generic_argument: syn::Type = f.type_ident();
                    ty_generics_tuple.elems.push_value(generic_argument.clone());
                    target_generics_tuple.elems.push_value(generic_argument);
                }
                ty_generics_tuple.elems.push_punct(Default::default());
                target_generics_tuple.elems.push_punct(Default::default());
            }
            generics
        };
        let mut target_generics = ty_generics.clone();
        target_generics.push(syn::GenericArgument::Type(target_generics_tuple.into()));
        ty_generics.push(syn::GenericArgument::Type(ty_generics_tuple.into()));
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let doc = field
            .builder_attr
            .doc
            .as_ref()
            .map(|doc| quote!(#[doc = #doc]));
        let deprecated = &field.builder_attr.deprecated;

        // // NOTE: both auto_into and strip_option affect `arg_type` and `arg_expr`, but the order of
        // // nesting is different so we have to do this little dance.
        // let arg_type = if field.builder_attr.strip_option.is_some() {
        //     field.type_from_inside_option().ok_or_else(|| {
        //         Error::new_spanned(
        //             field_type,
        //             "can't `strip_option` - field is not `Option<...>`",
        //         )
        //     })?
        // } else {
        //     field_type
        // };

        let arg_type = field_type;

        let (arg_type, arg_expr) = if field.builder_attr.auto_into.is_some() {
            (
                quote!(impl ::core::convert::Into<#arg_type>),
                quote!(#field_name.into()),
            )
        } else {
            (arg_type.to_token_stream(), field_name.to_token_stream())
        };

        // let (param_list, arg_expr) = if field.builder_attr.strip_option.is_some() {
        //     (quote!(#field_name: #arg_type), quote!(Some(#arg_expr)))
        // } else {
        //     (quote!(#field_name: #arg_type), arg_expr)
        // };

        let (param_list, arg_expr) = (quote!(#field_name: #arg_type), arg_expr);

        let repeated_fields_error_type_name = syn::Ident::new(
            &format!(
                "{}_Error_Repeated_field_{}",
                builder_name,
                strip_raw_ident_prefix(field_name.to_string())
            ),
            builder_name.span(),
        );
        let repeated_fields_error_message = format!("Repeated field {}", field_name);

        let method_name = field.name;

        Ok(quote_spanned! {
            self.builder_name.span() =>

            #[allow(dead_code, non_camel_case_types, missing_docs, clippy::type_complexity)]
            impl #impl_generics #builder_name < #( #ty_generics ),* > #where_clause {
                #deprecated
                #doc
                pub fn #method_name (self, #param_list) -> #builder_name <#( #target_generics ),*> {
                    let #field_name = (#arg_expr,);
                    let ( #(#descructuring,)* ) = self.fields;
                    #builder_name {
                        fields: ( #(#reconstructing,)* ),
                        phantom: self.phantom,
                    }
                }
            }
            #[doc(hidden)]
            #[allow(dead_code, non_camel_case_types, non_snake_case)]
            pub enum #repeated_fields_error_type_name {}
            #[doc(hidden)]
            #[allow(dead_code, non_camel_case_types, missing_docs, clippy::type_complexity)]
            impl #impl_generics #builder_name < #( #target_generics ),* > #where_clause {
                #[deprecated(
                    note = #repeated_fields_error_message
                )]
                pub fn #method_name (self, _: #repeated_fields_error_type_name) -> #builder_name <#( #target_generics ),*> {
                    self
                }
            }
        })
    }

    pub fn required_field_impl(&self, field: &FieldInfo) -> TokenStream {
        let agui_core = resolve_agui_path();

        let StructInfo {
            vis,
            ref builder_name,
            ..
        } = self;

        let FieldInfo {
            name: ref field_name,
            ..
        } = field;
        let mut builder_generics: Vec<syn::GenericArgument> = self
            .generics
            .params
            .iter()
            .map(|generic_param| match generic_param {
                syn::GenericParam::Type(type_param) => {
                    let ident = type_param.ident.to_token_stream();
                    syn::parse2(ident).unwrap()
                }
                syn::GenericParam::Lifetime(lifetime_def) => {
                    syn::GenericArgument::Lifetime(lifetime_def.lifetime.clone())
                }
                syn::GenericParam::Const(const_param) => {
                    let ident = const_param.ident.to_token_stream();
                    syn::parse2(ident).unwrap()
                }
            })
            .collect();
        let mut builder_generics_tuple = empty_type_tuple();
        let generics = {
            let mut generics = self.generics.clone();
            for f in self.included_fields() {
                if f.builder_attr.default.is_some() {
                    // `f` is not mandatory - it does not have it's own fake `build` method, so `field` will need
                    // to warn about missing `field` whether or not `f` is set.
                    assert!(
                        f.ordinal != field.ordinal,
                        "`required_field_impl` called for optional field {}",
                        field.name
                    );
                    generics.params.push(f.generic_ty_param());
                    builder_generics_tuple.elems.push_value(f.type_ident());
                } else if f.ordinal < field.ordinal {
                    // Only add a `build` method that warns about missing `field` if `f` is set. If `f` is not set,
                    // `f`'s `build` method will warn, since it appears earlier in the argument list.
                    builder_generics_tuple
                        .elems
                        .push_value(f.tuplized_type_ty_param());
                } else if f.ordinal == field.ordinal {
                    builder_generics_tuple.elems.push_value(empty_type());
                } else {
                    // `f` appears later in the argument list after `field`, so if they are both missing we will
                    // show a warning for `field` and not for `f` - which means this warning should appear whether
                    // or not `f` is set.
                    generics.params.push(f.generic_ty_param());
                    builder_generics_tuple.elems.push_value(f.type_ident());
                }

                builder_generics_tuple.elems.push_punct(Default::default());
            }
            generics
        };

        builder_generics.push(syn::GenericArgument::Type(builder_generics_tuple.into()));
        let (impl_generics, _, where_clause) = generics.split_for_impl();

        let early_build_error_type_name = syn::Ident::new(
            &format!(
                "{}_Error_Missing_required_field_{}",
                builder_name,
                strip_raw_ident_prefix(field_name.to_string())
            ),
            builder_name.span(),
        );

        // Marking build() as deprecated causes the whole `build!()` macro to be struct through visually,
        // which is extremely annoying. Removing it localizes the warning to the specific widget that is
        // missing a required field, which is significantly more useful.
        //
        // let early_build_error_message = format!("Missing required field {}", field_name);
        // #[deprecated(
        //     note = #early_build_error_message
        // )]

        quote_spanned! {
            builder_name.span() =>

            #[doc(hidden)]
            #[allow(dead_code, non_camel_case_types, non_snake_case)]
            pub enum #early_build_error_type_name {}
            #[doc(hidden)]
            #[allow(dead_code, non_camel_case_types, missing_docs, clippy::panic, clippy::type_complexity)]
            impl #impl_generics #builder_name < #( #builder_generics ),* > #where_clause {

                #vis fn build(self, _: #early_build_error_type_name) -> #agui_core::widget::Widget {
                    panic!()
                }
            }
        }
    }

    pub fn build_method_impl(&self) -> TokenStream {
        let agui_core = resolve_agui_path();

        let StructInfo {
            vis,
            ref name,
            ref builder_name,
            ..
        } = *self;

        let generics = {
            let mut generics = self.generics.clone();
            for field in self.included_fields() {
                if field.builder_attr.default.is_some() {
                    let trait_ref = syn::TraitBound {
                        paren_token: None,
                        lifetimes: None,
                        modifier: syn::TraitBoundModifier::None,
                        path: {
                            let mut path = agui_core.clone();
                            path.segments.push(syn::PathSegment {
                                ident: Ident::new("Optional", Span::call_site()),
                                arguments: syn::PathArguments::AngleBracketed(
                                    syn::AngleBracketedGenericArguments {
                                        colon2_token: None,
                                        lt_token: Default::default(),
                                        args: [syn::GenericArgument::Type(field.ty.clone())]
                                            .into_iter()
                                            .collect(),
                                        gt_token: Default::default(),
                                    },
                                ),
                            });
                            path
                        },
                    };
                    let mut generic_param: syn::TypeParam = field.generic_ident.clone().into();
                    generic_param.bounds.push(trait_ref.into());
                    generics.params.push(generic_param.into());
                }
            }
            generics
        };
        let (impl_generics, _, _) = generics.split_for_impl();

        let (_, ty_generics, where_clause) = self.generics.split_for_impl();

        let modified_ty_generics = modify_types_generics_hack(&ty_generics, |args| {
            args.push(syn::GenericArgument::Type(
                type_tuple(self.included_fields().map(|field| {
                    if field.builder_attr.default.is_some() {
                        field.type_ident()
                    } else {
                        field.tuplized_type_ty_param()
                    }
                }))
                .into(),
            ));
        });

        let descructuring = self.included_fields().map(|f| f.name);

        // The default of a field can refer to earlier-defined fields, which we handle by
        // writing out a bunch of `let` statements first, which can each refer to earlier ones.
        // This means that field ordering may actually be significant, which isn't ideal. We could
        // relax that restriction by calculating a DAG of field default dependencies and
        // reordering based on that, but for now this much simpler thing is a reasonable approach.
        let assignments = self.fields.iter().map(|field| {
            let name = &field.name;
            if let Some(ref default) = field.builder_attr.default {
                if field.builder_attr.skip.is_some() {
                    quote!(let #name = #default;)
                } else {
                    quote!(let #name = #agui_core::Optional::into_value(#name, || #default);)
                }
            } else {
                quote!(let #name = #name.0;)
            }
        });
        let field_names = self.fields.iter().map(|field| field.name);

        quote_spanned! {
            self.builder_name.span() =>

            #[allow(dead_code, non_camel_case_types, missing_docs, clippy::type_complexity)]
            impl #impl_generics #builder_name #modified_ty_generics #where_clause {
                #[allow(clippy::let_unit_value, clippy::default_trait_access)]
                #vis fn build(self) -> #name #ty_generics {
                    let ( #(#descructuring,)* ) = self.fields;
                    #( #assignments )*

                    #[allow(deprecated)]
                    #name {
                        #( #field_names ),*
                    }
                }
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct TypeBuilderAttr<'a> {
    pub field_defaults: FieldBuilderAttr<'a>,
}

impl<'a> TypeBuilderAttr<'a> {
    pub fn new(attrs: &[syn::Attribute]) -> Result<Self, Error> {
        let mut result = Self::default();

        for attr in attrs {
            let list = match &attr.meta {
                syn::Meta::List(list) => {
                    if path_to_single_string(&list.path).as_deref() != Some("props") {
                        continue;
                    }

                    list
                }

                _ => continue,
            };

            if list.tokens.is_empty() {
                return Err(syn::Error::new_spanned(list, "Expected props(…)"));
            }

            let parser = syn::punctuated::Punctuated::<_, syn::token::Comma>::parse_terminated;
            let exprs = parser.parse2(list.tokens.clone())?;
            for expr in exprs {
                result.field_defaults.apply_meta(expr)?;
            }
        }

        Ok(result)
    }
}
