#![allow(clippy::unwrap_used, clippy::expect_used)]
#![allow(dead_code)]

use std::{collections::HashMap, fs, path::PathBuf};

use heck::ToShoutySnakeCase;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields, Lit, LitStr, Type, parse_macro_input};

#[proc_macro_derive(EnumParam)]
pub fn derive_enum_param(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let Data::Enum(data_enum) = input.data else {
        panic!("EnumParam can only be derived for enums");
    };

    let variants: Vec<_> = data_enum.variants.into_iter().collect();

    // Extract variant identifiers and strings for use in generated code
    let variant_idents: Vec<_> = variants.iter().map(|v| v.ident.clone()).collect();
    let variant_strings: Vec<_> = variants.iter().map(|v| v.ident.to_string()).collect();

    // Fallback to the first variant if #[default] is missing
    let mut default_variant = variants
        .first()
        .expect("Enum must have at least one variant")
        .ident
        .clone();
    for variant in &variants {
        if variant
            .attrs
            .iter()
            .any(|attr| attr.path().is_ident("default"))
        {
            default_variant = variant.ident.clone();
            break;
        }
    }

    let expanded = quote! {
        impl ::karbeat_plugin_types::parameter::EnumParam for #name {
            #[inline(always)]
            fn to_index(self) -> usize {
                self as usize
            }

            #[inline(always)]
            fn from_index(index: usize) -> Self {
                // Generate an array of all variants
                let variants = [ #( #name::#variant_idents ),* ];

                if index < variants.len() {
                    variants[index]
                } else {
                    #name::#default_variant
                }
            }

            fn variants() -> &'static [&'static str] {
                // Generate the array of string slices
                &[ #( #variant_strings ),* ]
            }
        }
    };

    TokenStream::from(expanded)
}

struct ParamDef {
    field_name: syn::Ident,
    original_type: Type,
    id_str: String,
    name: String,
    group: String,
    min: f64,
    max: f64,
    step: f64,
    default: f64,
}

/// # Overview
///
/// Macro to generate implementation for getter, setter, and automation, and it also wrap your raw data type Item to Param<Item>
/// Therefore, you don't have to manually wrap all params with Param<Item>. This macro handles them for you
///
/// # How to Use
///
/// Put attribute macro on labelled parameters using `#[nested]` if
/// it is a non-primitive type, or in other words "custom type".
/// Else, just use the `#[param(id=your_id, name=your_name, group=your_group, min=your_min_value
/// max=your_max_value, default=your_default_value, step=your_step_value)]`.
/// For nested value, your custom type should also implement AutoParams. you can achieve
/// the same result by using the `#[karbeat_plugin]` macro again in your custom type
#[proc_macro_attribute]
pub fn karbeat_plugin(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(item as DeriveInput);
    let struct_name = &ast.ident;
    let enum_name = format_ident!("{}ParamIds", struct_name);
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    let mut params = Vec::new();
    let mut nested_fields: Vec<(syn::Ident, bool, String)> = Vec::new();
    let mut used_ids: HashMap<String, syn::Ident> = HashMap::new();
    if let Data::Struct(data_struct) = &mut ast.data
        && let Fields::Named(fields) = &mut data_struct.fields
    {
        for field in fields.named.iter_mut() {
            let field_ident = field.ident.clone().unwrap();
            let mut is_param = false;
            let mut is_nested = false;
            let mut has_serde_skip = false;
            let mut macro_error: Option<syn::Error> = None;

            for attr in &field.attrs {
                if attr.path().is_ident("param") {
                    is_param = true;
                    let mut p_id_str = String::new();
                    let mut p_name = String::new();
                    let mut p_group = String::new();
                    let mut p_min = 0.0;
                    let mut p_max = 1.0;
                    let mut p_default = 0.0;
                    let mut p_step = 0.0;

                    let res = attr.parse_nested_meta(|meta| {
                        if meta.path.is_ident("id") {
                            let value = meta.value()?.parse::<Lit>()?;
                            if let Lit::Str(lit_str) = value {
                                p_id_str = lit_str.value();

                                // Local string collision check!
                                if let Some(existing_field) = used_ids.get(&p_id_str) {
                                    return Err(
                                        syn::Error::new_spanned(
                                            &lit_str,
                                            format!(
                                                "Parameter ID collision! Local ID `{}` is already used by field `{}`.",
                                                p_id_str,
                                                existing_field
                                            )
                                        )
                                    );
                                }
                                used_ids.insert(p_id_str.clone(), field_ident.clone());
                            }
                        } else if meta.path.is_ident("name") {
                            let value = meta.value()?.parse::<Lit>()?;
                            if let Lit::Str(lit_str) = value {
                                p_name = lit_str.value();
                            }
                        } else if meta.path.is_ident("group") {
                            let value = meta.value()?.parse::<Lit>()?;
                            if let Lit::Str(lit_str) = value {
                                p_group = lit_str.value();
                            }
                        } else if meta.path.is_ident("min") {
                            let value = meta.value()?.parse::<Lit>()?;
                            if let Lit::Float(lit_float) = value {
                                p_min = lit_float.base10_parse()?;
                            }
                        } else if meta.path.is_ident("max") {
                            let value = meta.value()?.parse::<Lit>()?;
                            if let Lit::Float(lit_float) = value {
                                p_max = lit_float.base10_parse()?;
                            }
                        } else if meta.path.is_ident("default") {
                            let value = meta.value()?.parse::<Lit>()?;
                            if let Lit::Float(lit_float) = value {
                                p_default = lit_float.base10_parse()?;
                            }
                        } else if meta.path.is_ident("step") {
                            let value = meta.value()?.parse::<Lit>()?;
                            if let Lit::Float(lit_float) = value {
                                p_step = lit_float.base10_parse()?;
                            }
                        } else {
                            return Err(
                                syn::Error::new_spanned(
                                    &meta.path,
                                    format!("{:?} is not a valid parameter", meta.path)
                                )
                            );
                        }
                        Ok(())
                    });

                    if let Err(e) = res {
                        macro_error = Some(e);
                    }

                    params.push(ParamDef {
                        field_name: field_ident.clone(),
                        original_type: field.ty.clone(),
                        id_str: p_id_str,
                        name: p_name,
                        group: p_group,
                        min: p_min,
                        max: p_max,
                        step: p_step,
                        default: p_default,
                    });
                } else if attr.path().is_ident("nested") {
                    is_nested = true;
                    let mut n_prefix = String::new();

                    // Parse prefix attribute if provided
                    let _ = attr.parse_nested_meta(|meta| {
                        if meta.path.is_ident("prefix")
                            && let Lit::Str(lit_str) = meta.value()?.parse::<Lit>()?
                        {
                            n_prefix = lit_str.value();
                        }
                        Ok(())
                    });

                    let is_iterable = match &field.ty {
                        Type::Array(_) | Type::Slice(_) => true,
                        Type::Path(p) => {
                            if let Some(segment) = p.path.segments.last() {
                                let id = segment.ident.to_string();
                                id == "Vec" || id == "VecDeque" || id == "Option"
                            } else {
                                false
                            }
                        }
                        _ => false,
                    };
                    nested_fields.push((field_ident.clone(), is_iterable, n_prefix));
                } else if attr.path().is_ident("serde") {
                    // Check if the user manually added #[serde(skip)]
                    let _ = attr.parse_nested_meta(|meta| {
                        if meta.path.is_ident("skip") {
                            has_serde_skip = true;
                        }
                        Ok(())
                    });
                }
            }

            if let Some(err) = macro_error {
                return TokenStream::from(err.to_compile_error());
            }

            if is_param {
                let orig_ty = &field.ty;
                let new_ty: Type = syn::parse_quote!(::karbeat_plugin_types::Param<#orig_ty>);
                field.ty = new_ty;
            }

            if !is_param && !is_nested && !has_serde_skip {
                field.attrs.push(syn::parse_quote!(#[serde(skip)]));
            }

            // Remove the custom attributes so the Rust compiler doesn't panic
            field
                .attrs
                .retain(|attr| !attr.path().is_ident("param") && !attr.path().is_ident("nested"));
        }
    }

    let enum_variants = params.iter().map(|p| {
        let variant_name = format_ident!("{}", p.name.replace(" ", ""));
        let id = &p.id_str;
        quote! { #variant_name = ::karbeat_utils::hash::hash_str(#id) }
    });

    let set_match_arms = params.iter().map(|p| {
        let field = &p.field_name;
        let id_str = &p.id_str;
        let ty = &p.original_type;
        quote! {
            if id == ::karbeat_utils::hash::hash_str_from(prefix_hash, #id_str) {
                let any_val = &value as &dyn ::std::any::Any;
                if let ::core::option::Option::Some(v) = any_val.downcast_ref::<#ty>() {
                    self.#field.set_base(*v);
                } else {
                    self.#field.set_base_from_f64(value.to_f64());
                }
                return true;
            }
        }
    });

    let nested_set_stmts = nested_fields.iter().map(|(f, is_iterable, prefix)| {
        if *is_iterable {
            quote! {
                for (i, item) in self.#f.iter_mut().enumerate() {
                    let child_prefix = format!("{}{}/", #prefix, i);
                    let child_hash = karbeat_utils::hash::hash_str_from(prefix_hash, &child_prefix);
                    if item.auto_set_parameter(child_hash, id, value) {
                        return true;
                    }
                }
            }
        } else {
            quote! {
                let child_hash = ::karbeat_utils::hash::hash_str_from(prefix_hash, #prefix);
                if self.#f.auto_set_parameter(child_hash, id, value) {
                    return true;
                }
            }
        }
    });

    let get_match_arms = params.iter().map(|p| {
        let field = &p.field_name;
        let id_str = &p.id_str;

        quote! {
            if id == ::karbeat_utils::hash::hash_str_from(prefix_hash, #id_str) {
                let val = self.#field.get_base();
                let any_val = &val as &dyn ::std::any::Any;
                if let ::core::option::Option::Some(v) = any_val.downcast_ref::<V>() {
                    return ::core::option::Option::Some(*v);
                } else {
                    return ::core::option::Option::Some(V::from_f64(val.to_f64()));
                }
            }
        }
    });

    let nested_get_stmts = nested_fields.iter().map(|(f, is_iterable, prefix)| {
        if *is_iterable {
            quote! {
                for (i, item) in self.#f.iter().enumerate() {
                    let child_prefix = format!("{}{}/", #prefix, i);
                    let child_hash = ::karbeat_utils::hash::hash_str_from(prefix_hash, &child_prefix);
                    if let Some(v) = item.auto_get_parameter(child_hash, id) {
                        return Some(v);
                    }
                }
            }
        } else {
            quote! {
                let child_hash = ::karbeat_utils::hash::hash_str_from(prefix_hash, #prefix);
                if let Some(v) = self.#f.auto_get_parameter(child_hash, id) {
                    return Some(v);
                }
            }
        }
    });

    let apply_auto_arms = params.iter().map(|p| {
        let field = &p.field_name;
        let id_str = &p.id_str;
        let ty = &p.original_type;
        quote! {
            if id == ::karbeat_utils::hash::hash_str_from(prefix_hash, #id_str) {
                let any_val = &value as &dyn ::std::any::Any;
                if let ::core::option::Option::Some(v) = any_val.downcast_ref::<#ty>() {
                    self.#field.apply_automation(*v);
                } else {
                    self.#field.apply_automation_from_f64(value.to_f64());
                }
                return true;
            }
        }
    });

    let nested_apply_stmts = nested_fields.iter().map(|(f, is_iterable, prefix)| {
        if *is_iterable {
            quote! {
                for (i, item) in self.#f.iter_mut().enumerate() {
                    let child_prefix = format!("{}{}/", #prefix, i);
                    let child_hash = ::karbeat_utils::hash::hash_str_from(prefix_hash, &child_prefix);
                    if item.auto_apply_automation(child_hash, id, value) {
                        return true;
                    }
                }
            }
        } else {
            quote! {
                let child_hash = ::karbeat_utils::hash::hash_str_from(prefix_hash, #prefix);
                if self.#f.auto_apply_automation(child_hash, id, value) {
                    return true;
                }
            }
        }
    });

    let clear_auto_arms = params.iter().map(|p| {
        let field = &p.field_name;
        let id_str = &p.id_str;
        quote! {
            if id == ::karbeat_utils::hash::hash_str_from(prefix_hash, #id_str) {
                self.#field.clear_automation();
                return true;
            }
        }
    });

    let nested_clear_stmts = nested_fields.iter().map(|(f, is_iterable, prefix)| {
        if *is_iterable {
            quote! {
                for (i, item) in self.#f.iter_mut().enumerate() {
                    let child_prefix = format!("{}{}/", #prefix, i);
                    let child_hash = ::karbeat_utils::hash::hash_str_from(prefix_hash, &child_prefix);
                    if item.auto_clear_automation(child_hash, id) {
                        return true;
                    }
                }
            }
        } else {
            quote! {
                let child_hash = ::karbeat_utils::hash::hash_str_from(prefix_hash, #prefix);
                if self.#f.auto_clear_automation(child_hash, id) {
                    return true;
                }
            }
        }
    });

    // When returning specs, dynamically calculate the globally unique ID
    let spec_pushes = params.iter().map(|p| {
        let field = &p.field_name;
        let id_str = &p.id_str;
        quote! {
            let mut spec = self.#field.to_spec();
            spec.id = ::karbeat_utils::hash::hash_str_from(prefix_hash, #id_str);
            // Concatenate the parent prefix with the local ID
            spec.path = ::std::format!("{}{}", prefix_str, #id_str);
            specs.push(spec);
        }
    });

    let nested_spec_stmts = nested_fields.iter().map(|(f, is_iterable, prefix)| {
        if *is_iterable {
            quote! {
                for (i, item) in self.#f.iter().enumerate() {
                    let child_prefix = ::std::format!("{}{}/", #prefix, i);
                    let child_hash = ::karbeat_utils::hash::hash_str_from(prefix_hash, &child_prefix);
                    // Build the new string prefix for the child
                    let child_str = ::std::format!("{}{}", prefix_str, child_prefix);
                    specs.extend(item.auto_get_parameter_specs(child_hash, &child_str));
                }
            }
        } else {
            quote! {
                let child_hash = ::karbeat_utils::hash::hash_str_from(prefix_hash, #prefix);
                let child_str = ::std::format!("{}{}", prefix_str, #prefix);
                specs.extend(self.#f.auto_get_parameter_specs(child_hash, &child_str));
            }
        }
    });

    let type_assertions = params.iter().map(|p| {
        let ty = &p.original_type;
        quote! {
            let _ = assert_implements_param_type::<#ty>;
        }
    });

    let mut default_field_inits = Vec::new();

    if let Data::Struct(data_struct) = &ast.data
        && let Fields::Named(fields) = &data_struct.fields
    {
        for field in fields.named.iter() {
            let field_ident = field.ident.as_ref().unwrap();

            // If the field is in our parsed parameters list, initialize it fully
            if let Some(p) = params.iter().find(|p| &p.field_name == field_ident) {
                let id = &p.id_str;
                let name = &p.name;
                let group = &p.group;
                let default_val = p.default;
                let min = p.min;
                let max = p.max;
                let step = p.step;
                let ty = &p.original_type;

                // Convert the AST Type to a string to check what it is
                let type_ident = if let syn::Type::Path(type_path) = ty {
                    type_path
                        .path
                        .segments
                        .last()
                        .map(|s| s.ident.to_string())
                        .unwrap_or_default()
                } else {
                    "".to_string()
                };

                let local_hash = quote! { ::karbeat_utils::hash::hash_str(#id) };

                let param_init = if type_ident == "f32" {
                    // Cast to f32 BEFORE quote! so it generates the `f32` literal suffix natively
                    let default_f32 = default_val as f32;
                    let min_f32 = min as f32;
                    let max_f32 = max as f32;
                    let step_f32 = step as f32;

                    quote! {
                        ::karbeat_plugin_types::parameter::Param::new_f32(#local_hash, #name, #group, #default_f32, #min_f32, #max_f32, #step_f32)
                    }
                } else if type_ident == "f64" {
                    quote! {
                        ::karbeat_plugin_types::parameter::Param::new_f64(#local_hash, #name, #group, #default_val, #min, #max, #step)
                    }
                } else if type_ident == "i32" {
                    let default_i32 = default_val as i32;
                    let min_i32 = min as i32;
                    let max_i32 = max as i32;
                    let step_i32 = step as i32;

                    quote! {
                        ::karbeat_plugin_types::parameter::Param::new_i32(#local_hash, #name, #group, #default_i32, #min_i32, #max_i32, #step_i32)
                    }
                } else if type_ident == "bool" {
                    let default_bool = default_val >= 0.5;
                    quote! {
                        ::karbeat_plugin_types::parameter::Param::new_bool(#local_hash, #name, #group, #default_bool)
                    }
                } else {
                    let default_idx = default_val as usize;
                    quote! {
                        ::karbeat_plugin_types::parameter::Param::<#ty>::new_enum(
                            #local_hash,
                            #name,
                            #group,
                            <#ty as ::karbeat_plugin_types::parameter::EnumParam>::from_index(#default_idx)
                        )
                    }
                };

                default_field_inits.push(quote! {
                    #field_ident: #param_init
                });
            } else {
                // For #[nested] or standard fields, fallback to standard default
                default_field_inits.push(quote! {
                    #field_ident: std::default::Default::default()
                });
            }
        }
    }

    // Conditionally generate the enum only if there are parameters
    let enum_definition = if params.is_empty() {
        quote! {}
    } else {
        quote! {
            #[repr(u32)]
            pub enum #enum_name {
                #(#enum_variants),*
            }
        }
    };

    let expanded = quote! {
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #ast

        #enum_definition

       impl #impl_generics #struct_name #ty_generics #where_clause {
            /// Creates an instance with all `#[param]` fields initialized to their macro defaults.
            pub fn base_default() -> Self {
                fn assert_implements_param_type<TypeToCheck: ::karbeat_plugin_types::parameter::ParamType>() {}
                #(#type_assertions)*

                Self {
                    #(#default_field_inits),*
                }
            }
        }

        const _: () = {
            use karbeat_plugin_types::*;
            impl #impl_generics AutoParams for #struct_name #ty_generics #where_clause {
                fn auto_set_parameter<V: ::karbeat_plugin_types::parameter::ParamType>(&mut self, prefix_hash: u32, id: u32, value: V) -> bool {
                    #(#set_match_arms)*
                    #(#nested_set_stmts)*
                    false
                }

                fn auto_get_parameter<V: ::karbeat_plugin_types::parameter::ParamType>(&self, prefix_hash: u32, id: u32) -> Option<V> {
                    #(#get_match_arms)*
                    #(#nested_get_stmts)*
                    None
                }

                fn auto_apply_automation<V: ::karbeat_plugin_types::parameter::ParamType>(&mut self, prefix_hash: u32, id: u32, value: V) -> bool {
                    #(#apply_auto_arms)*
                    #(#nested_apply_stmts)*
                    false
                }

                fn auto_clear_automation(&mut self, prefix_hash: u32, id: u32) -> bool {
                    #(#clear_auto_arms)*
                    #(#nested_clear_stmts)*
                    false
                }

                fn auto_get_parameter_specs(&self, prefix_hash: u32, prefix_str: &str) -> Vec<karbeat_plugin_types::parameter::ParameterSpec> {
                    let mut specs = Vec::new();
                    #(#spec_pushes)*
                    #(#nested_spec_stmts)*
                    specs
                }
            }
        };
    };
    TokenStream::from(expanded)
}

#[proc_macro_derive(AutoParams, attributes(skip, param))]
pub fn derive_auto_params(input: TokenStream) -> TokenStream {
    let input_derive = parse_macro_input!(input as DeriveInput);

    let name = &input_derive.ident;

    let (impl_generics, ty_generics, where_clause) = input_derive.generics.split_for_impl();
    let fields = match &input_derive.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(fields_named) => &fields_named.named,
            _ => panic!("AutoParams can only be derived on structs with named fields"),
        },
        _ => panic!("AutoParams can only be derived on structs"),
    };

    // We now store a tuple of (Field, StringID, InnerType)
    let mut param_fields = Vec::new();

    for field in fields.iter() {
        let is_nested = field.attrs.iter().any(|attr| attr.path().is_ident("skip"));

        if !is_nested {
            let mut is_valid_param = false;
            let mut inner_ty = None;

            if let Type::Path(type_path) = &field.ty
                && let Some(segment) = type_path.path.segments.last()
                && segment.ident == "Param"
            {
                is_valid_param = true;
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(ty)) = args.args.first() {
                        inner_ty = Some(ty.clone());
                    }
                }
            }

            if !is_valid_param {
                let error_msg = format!(
                    "AutoParams requires field '{}' to be of type `karbeat_plugin_types::Param<T>`. \n\
                    If this is a sub-module or standard field, ignore it with `#[skip]`.",
                    field.ident.as_ref().unwrap()
                );

                return syn::Error::new_spanned(&field.ty, error_msg)
                    .to_compile_error()
                    .into();
            }

            // Default the ID string to the exact field name
            let mut id_str = field.ident.as_ref().unwrap().to_string();

            // Check if the user overrode the ID with #[param(id = "custom_id")]
            for attr in &field.attrs {
                if attr.path().is_ident("param") {
                    let _ = attr.parse_nested_meta(|meta| {
                        if meta.path.is_ident("id")
                            && let Ok(syn::Lit::Str(lit_str)) =
                                meta.value().and_then(|v| v.parse::<syn::Lit>())
                        {
                            id_str = lit_str.value();
                        }
                        Ok(())
                    });
                }
            }

            param_fields.push((field, id_str, inner_ty));
        }
    }

    // Fully qualify paths to ensure absolute hygiene
    let get_arms = param_fields.iter().map(|(f, id_str, _)| {
        let fname = &f.ident;
        quote! {
            if id == ::karbeat_utils::hash::hash_str_from(prefix_hash, #id_str) {
                let val = self.#fname.get_base();
                let any_val = &val as &dyn ::std::any::Any;
                if let ::core::option::Option::Some(v) = any_val.downcast_ref::<V>() {
                    return ::core::option::Option::Some(*v);
                } else {
                    return ::core::option::Option::Some(V::from_f64(val.to_f64()));
                }
            }
        }
    });

    let set_arms = param_fields.iter().map(|(f, id_str, inner_ty)| {
        let fname = &f.ident;
        let ty = inner_ty.as_ref().unwrap();
        quote! {
            if id == ::karbeat_utils::hash::hash_str_from(prefix_hash, #id_str) {
                let any_val = &value as &dyn ::std::any::Any;
                if let ::core::option::Option::Some(v) = any_val.downcast_ref::<#ty>() {
                    self.#fname.set_base(*v);
                } else {
                    self.#fname.set_base_from_f64(value.to_f64());
                }
                return true;
            }
        }
    });

    let apply_arms = param_fields.iter().map(|(f, id_str, inner_ty)| {
        let fname = &f.ident;
        let ty = inner_ty.as_ref().unwrap();
        quote! {
            if id == ::karbeat_utils::hash::hash_str_from(prefix_hash, #id_str) {
                let any_val = &value as &dyn ::std::any::Any;
                if let ::core::option::Option::Some(v) = any_val.downcast_ref::<#ty>() {
                    self.#fname.apply_automation(*v);
                } else {
                    self.#fname.apply_automation_from_f64(value.to_f64());
                }
                return true;
            }
        }
    });

    let clear_arms = param_fields.iter().map(|(f, id_str, _)| {
        let fname = &f.ident;
        quote! {
            if id == ::karbeat_utils::hash::hash_str_from(prefix_hash, #id_str) {
                self.#fname.clear_automation();
                return true;
            }
        }
    });

    let spec_arms = param_fields.iter().map(|(f, id_str, _)| {
        let fname = &f.ident;
        quote! {
            {
                let mut spec = self.#fname.to_spec();
                // Overwrite the local ID with the fully resolved, globally unique hash
                spec.id = ::karbeat_utils::hash::hash_str_from(prefix_hash, #id_str);
                spec
            }
        }
    });

    let expanded = quote! {
        const _: () = {
            // Hygienically target the specific traits
            use ::karbeat_plugin_types::*;
            impl #impl_generics ::karbeat_plugin_types::parameter::AutoParams for #name #ty_generics #where_clause {
                fn auto_get_parameter<V: ::karbeat_plugin_types::parameter::ParamType>(&self, prefix_hash: u32, id: u32) -> ::core::option::Option<V> {
                    #(#get_arms)*
                    ::core::option::Option::None
                }

                fn auto_set_parameter<V: ::karbeat_plugin_types::parameter::ParamType>(&mut self, prefix_hash: u32, id: u32, value: V) -> bool {
                    #(#set_arms)*
                    false
                }

                fn auto_apply_automation<V: ::karbeat_plugin_types::parameter::ParamType>(&mut self, prefix_hash: u32, id: u32, value: V) -> bool {
                    #(#apply_arms)*
                    false
                }

                fn auto_clear_automation(&mut self, prefix_hash: u32, id: u32) -> bool {
                    #(#clear_arms)*
                    false
                }

                fn auto_get_parameter_specs(&self, prefix_hash: u32) -> ::std::vec::Vec<::karbeat_plugin_types::parameter::ParameterSpec> {
                    ::std::vec![
                        #(#spec_arms),*
                    ]
                }
            }
        };
    };

    TokenStream::from(expanded)
}

#[proc_macro]
pub fn build_dynamic_registry(input: TokenStream) -> TokenStream {
    let dir_lit = parse_macro_input!(input as LitStr);
    let rel_dir = dir_lit.value();

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let base_path = PathBuf::from(manifest_dir).join(&rel_dir);

    let synths_path = base_path.join("synths");
    let effects_path = base_path.join("effects");

    let mut generator_inserts = Vec::new();
    let mut effect_inserts = Vec::new();
    let mut file_trackers = Vec::new();
    let mut associated_constants = Vec::new(); // <-- NEW: Store the generated constants

    let mut process_directory = |dir_path: &PathBuf, is_synth: bool| {
        if let Ok(entries) = fs::read_dir(dir_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) != Some("json") {
                    continue;
                }

                let json_data = fs::read_to_string(&path).unwrap();
                let manifest: serde_json::Value = serde_json::from_str(&json_data)
                    .unwrap_or_else(|_| panic!("Invalid JSON in {:?}", path));

                let id = manifest["id"].as_u64().expect("Missing id") as u32;
                let name = manifest["name"].as_str().expect("Missing name");

                let internal_type_str = manifest["internal_type"]
                    .as_str()
                    .expect("Missing 'internal_type' in manifest!");
                let internal_ident = format_ident!("{}", internal_type_str);

                let shouty_name = internal_type_str.to_owned().to_shouty_snake_case();
                let const_prefix = if is_synth { "GEN_" } else { "EFF_" };
                let const_ident = format_ident!("{}{}", const_prefix, shouty_name);

                associated_constants.push(quote! {
                    pub const #const_ident: u32 = #id;
                });

                let mut param_tokens = Vec::new();
                if let Some(params) = manifest["parameters"].as_array() {
                    for p in params {
                        let p_id = p["id"].as_u64().unwrap() as u32;
                        let p_name = p["name"].as_str().unwrap();
                        let p_group = p["group"].as_str().unwrap();
                        let default_val = p["default_value"].as_f64().unwrap() as f32;
                        let min = p["min"].as_f64().unwrap() as f32;
                        let max = p["max"].as_f64().unwrap() as f32;
                        let step = p["step"].as_f64().unwrap() as f32;

                        let val_type_str = p["value_type"].as_str().unwrap();
                        let val_type_ident = format_ident!("{}", val_type_str);

                        let choices_tokens = if let Some(choices) = p["choices"].as_array() {
                            let c_strs = choices.iter().map(|c| c.as_str().unwrap());
                            quote! { vec![ #(#c_strs.to_string()),* ] }
                        } else {
                            quote! { vec![] }
                        };

                        param_tokens.push(quote! {
                            karbeat_plugin_types::ParameterSpec {
                                id: #p_id,
                                name: #p_name.to_string(),
                                group: #p_group.to_string(),
                                default_value: #default_val,
                                min: #min,
                                max: #max,
                                step: #step,
                                value_type: karbeat_plugin_types::ParamType::#val_type_ident,
                                choices: #choices_tokens,
                            }
                        });
                    }
                }

                let abs_path_str = path.to_str().unwrap();
                file_trackers.push(quote! {
                    const _: &[u8] = include_bytes!(#abs_path_str);
                });

                let insert_stmt = quote! {
                    let specs = vec![ #(#param_tokens),* ];
                    let factory: Box<dyn Fn() -> Box<_> + Send + Sync> = Box::new(|| Box::new( <#internal_ident>::build() ));

                    let registered = RegisteredPlugin {
                        name: #name.to_string(),
                        factory,
                        parameter_specs: specs,
                    };
                };

                if is_synth {
                    generator_inserts.push(quote! {
                        #insert_stmt
                        registry.generators.insert(#id, registered);
                    });
                } else {
                    effect_inserts.push(quote! {
                        #insert_stmt
                        registry.effects.insert(#id, registered);
                    });
                }
            }
        }
    };

    process_directory(&synths_path, true);
    process_directory(&effects_path, false);

    // 2. Assemble the final output, now including the constants!
    let expanded = quote! {
        // Automatically inject the compiled constants
        #(#associated_constants)*

        pub fn new_with_defaults() -> Self {
            let mut registry = Self::new();

            // Inject file trackers to ensure macro hygiene and hot-reloading
            #(#file_trackers)*

            // Inject the dynamic bindings
            #(#generator_inserts)*
            #(#effect_inserts)*

            registry
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn auto_param(args: TokenStream, item: TokenStream) -> TokenStream {
    let mut on_change_expr: Option<proc_macro2::TokenStream> = None;

    if !args.is_empty() {
        let meta_parser = syn::meta::parser(|meta| {
            if meta.path.is_ident("on_change") {
                let lit: syn::LitStr = meta.value()?.parse()?;
                let expr: syn::Expr = lit.parse()?;
                on_change_expr = Some(quote::quote!(#expr));
                Ok(())
            } else {
                Err(meta.error("unsupported property"))
            }
        });
        syn::parse_macro_input!(args with meta_parser);
    }

    let mut item_impl = syn::parse_macro_input!(item as syn::ItemImpl);
    let self_ty = &item_impl.self_ty;

    // Check if methods are already implemented
    let mut has_set_parameter = false;
    let mut has_get_parameter = false;
    let mut has_apply_automation = false;
    let mut has_clear_automation = false;
    let mut has_default_parameters = false;
    let mut has_get_parameter_specs = false;
    let mut has_static_parameter_specs = false;
    let mut has_as_any = false;

    for item in &item_impl.items {
        if let syn::ImplItem::Fn(method) = item {
            let name = &method.sig.ident;
            if name == "set_parameter" {
                has_set_parameter = true;
            }
            if name == "get_parameter" {
                has_get_parameter = true;
            }
            if name == "apply_automation" {
                has_apply_automation = true;
            }
            if name == "clear_automation" {
                has_clear_automation = true;
            }
            if name == "default_parameters" {
                has_default_parameters = true;
            }
            if name == "get_parameter_specs" {
                has_get_parameter_specs = true;
            }
            if name == "static_parameter_specs" {
                has_static_parameter_specs = true;
            }
            if name == "as_any" {
                has_as_any = true;
            }
        }
    }

    let on_change = if let Some(expr) = on_change_expr {
        quote::quote! { #expr; }
    } else {
        quote::quote! {}
    };

    if !has_set_parameter {
        let method: syn::ImplItemFn = syn::parse_quote! {
            fn set_parameter(&mut self, id: u32, value: f32) {
                if self.auto_set_parameter(::karbeat_utils::hash::FNV_OFFSET, id, value) {
                    #on_change
                }
            }
        };
        item_impl.items.push(syn::ImplItem::Fn(method));
    }

    if !has_get_parameter {
        let method: syn::ImplItemFn = syn::parse_quote! {
            fn get_parameter(&self, id: u32) -> f32 {
                self.auto_get_parameter(::karbeat_utils::hash::FNV_OFFSET, id).unwrap_or(0.0)
            }
        };
        item_impl.items.push(syn::ImplItem::Fn(method));
    }

    if !has_apply_automation {
        let method: syn::ImplItemFn = syn::parse_quote! {
            fn apply_automation(&mut self, id: u32, value: f32) {
                if self.auto_apply_automation(::karbeat_utils::hash::FNV_OFFSET, id, value) {
                    #on_change
                }
            }
        };
        item_impl.items.push(syn::ImplItem::Fn(method));
    }

    if !has_clear_automation {
        let method: syn::ImplItemFn = syn::parse_quote! {
            fn clear_automation(&mut self, id: u32) {
                if self.auto_clear_automation(::karbeat_utils::hash::FNV_OFFSET, id) {
                    #on_change
                }
            }
        };
        item_impl.items.push(syn::ImplItem::Fn(method));
    }

    if !has_default_parameters {
        let method: syn::ImplItemFn = syn::parse_quote! {
            fn default_parameters(&self) -> ::hashbrown::HashMap<u32, f32> {
                self.auto_get_parameter_specs(::karbeat_utils::hash::FNV_OFFSET, "")
                    .into_iter()
                    .map(|spec| (spec.id, spec.default_value as f32))
                    .collect()
            }
        };
        item_impl.items.push(syn::ImplItem::Fn(method));
    }

    if !has_get_parameter_specs {
        let method: syn::ImplItemFn = syn::parse_quote! {
            fn get_parameter_specs(&self) -> Vec<::karbeat_plugin_types::ParameterSpec> {
                self.auto_get_parameter_specs(::karbeat_utils::hash::FNV_OFFSET, "")
            }
        };
        item_impl.items.push(syn::ImplItem::Fn(method));
    }

    if !has_static_parameter_specs {
        let method: syn::ImplItemFn = syn::parse_quote! {
            fn static_parameter_specs() -> Vec<::karbeat_plugin_types::ParameterSpec>
            where
                Self: Sized,
            {
                <#self_ty>::default().auto_get_parameter_specs(::karbeat_utils::hash::FNV_OFFSET, "")
            }
        };
        item_impl.items.push(syn::ImplItem::Fn(method));
    }

    if !has_as_any {
        let method: syn::ImplItemFn = syn::parse_quote! {
            fn as_any(&self) -> &dyn ::std::any::Any {
                self
            }
        };
        item_impl.items.push(syn::ImplItem::Fn(method));
    }

    let expanded = quote::quote! {
        const _: () = {
            use ::karbeat_plugin_types::parameter::AutoParams;
            #item_impl
        };
    };

    expanded.into()
}
