//! `#[derive(GardeSchema)]` and `#[derive(ValidatorSchema)]` — auto-generate
//! `JsonSchema` impls that preserve `schemars` behavior and inject static
//! validation constraints.
//!
//! The generated impl delegates baseline schema generation to a hidden helper
//! struct derived with `::schemars::JsonSchema`, then patches validator/garde
//! constraints into the resulting schema.

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::parse::Parser;
use syn::{Attribute, DeriveInput, Expr, Field, Fields, Lit};

#[derive(Default)]
struct SchemaConstraints {
    required: bool,
    min_length: Option<u64>,
    max_length: Option<u64>,
    minimum: Option<NumericLit>,
    maximum: Option<NumericLit>,
    format: Option<String>,
    pattern: Option<String>,
}

#[derive(Clone)]
enum NumericLit {
    Int(i64),
    Float(f64),
}

impl SchemaConstraints {
    fn is_empty(&self) -> bool {
        self.min_length.is_none()
            && self.max_length.is_none()
            && self.minimum.is_none()
            && self.maximum.is_none()
            && self.format.is_none()
            && self.pattern.is_none()
            && !self.required
    }
}

struct FieldInfo {
    schema_name: String,
    field: Field,
    constraints: SchemaConstraints,
}

#[derive(Clone, Copy)]
enum RenameRule {
    Lower,
    Upper,
    Pascal,
    Camel,
    Snake,
    ScreamingSnake,
    Kebab,
    ScreamingKebab,
}

fn collect_field_info<F>(
    fields: &Fields,
    validation_attr: &str,
    rename_all: Option<RenameRule>,
    parse_constraints: F,
) -> syn::Result<Vec<FieldInfo>>
where
    F: Fn(&[Attribute]) -> SchemaConstraints,
{
    let named = match fields {
        Fields::Named(named) => named,
        _ => {
            return Err(syn::Error::new_spanned(
                fields,
                "derive macro requires named fields",
            ))
        }
    };

    Ok(named
        .named
        .iter()
        .map(|field| {
            let schema_name = schema_field_name(field, rename_all);
            let constraints = parse_constraints(&field.attrs);

            let mut cloned = field.clone();
            cloned.attrs = filter_attrs(&field.attrs, validation_attr);

            FieldInfo {
                schema_name,
                field: cloned,
                constraints,
            }
        })
        .collect())
}

fn filter_attrs(attrs: &[Attribute], validation_attr: &str) -> Vec<Attribute> {
    attrs.iter()
        .filter(|attr| {
            let path = attr.path();
            !path.is_ident(validation_attr) && !path.is_ident("derive")
        })
        .cloned()
        .collect()
}

fn schema_field_name(field: &Field, rename_all: Option<RenameRule>) -> String {
    let fallback = field.ident.as_ref().unwrap().to_string();

    field_rename_from_attr(&field.attrs, "schemars")
        .or_else(|| field_rename_from_attr(&field.attrs, "serde"))
        .or_else(|| rename_all.map(|rule| apply_rename_rule(&fallback, rule)))
        .unwrap_or(fallback)
}

fn field_rename_from_attr(attrs: &[Attribute], attr_name: &str) -> Option<String> {
    for attr in attrs {
        if !attr.path().is_ident(attr_name) {
            continue;
        }

        let Ok(nested) = attr.parse_args_with(
            syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
        ) else {
            continue;
        };

        for meta in &nested {
            if let syn::Meta::NameValue(nv) = meta {
                if nv.path.is_ident("rename") {
                    if let Some(value) = expr_to_str(&nv.value) {
                        return Some(value);
                    }
                }
            }
        }
    }

    None
}

fn expand_derive_with<F>(
    input: DeriveInput,
    macro_name: &str,
    validation_attr: &str,
    parser: F,
) -> syn::Result<TokenStream>
where
    F: Fn(&[Attribute]) -> SchemaConstraints,
{
    let struct_ident = input.ident;
    let generics = input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let data = match input.data {
        syn::Data::Struct(data) => data,
        _ => {
            return Err(syn::Error::new_spanned(
                &struct_ident,
                format!("{macro_name} can only be derived for structs"),
            ))
        }
    };

    let rename_all = struct_rename_all(&input.attrs);
    let struct_attrs = filter_attrs(&input.attrs, validation_attr);
    let field_info = collect_field_info(&data.fields, validation_attr, rename_all, parser)?;
    let helper_ident = format_ident!("__SummerSchemaBase_{}", struct_ident);

    let helper_struct = generate_helper_struct(
        &helper_ident,
        &struct_attrs,
        &data.fields,
        &field_info,
        &generics,
    )?;

    let patch_stmts = field_info
        .iter()
        .filter(|field| !field.constraints.is_empty())
        .map(|field| {
            let field_name = &field.schema_name;
            let stmts = generate_constraint_stmts(&field.constraints);
            quote! {
                if let Some(__prop) = __props.get_mut(#field_name) {
                    if let Some(__prop_obj) = __prop.as_object_mut() {
                        #stmts
                    }
                }
            }
        });

    let required_fields = field_info
        .iter()
        .filter(|field| field.constraints.required)
        .map(|field| field.schema_name.as_str());

    let helper_name = helper_ident.to_string();
    let original_name = struct_ident.to_string();

    Ok(quote! {
        #helper_struct

        impl #impl_generics ::schemars::JsonSchema for #struct_ident #ty_generics #where_clause {
            fn inline_schema() -> bool {
                <#helper_ident #ty_generics as ::schemars::JsonSchema>::inline_schema()
            }

            fn schema_name() -> ::std::borrow::Cow<'static, str> {
                let helper = <#helper_ident #ty_generics as ::schemars::JsonSchema>::schema_name()
                    .into_owned();
                helper.replace(#helper_name, #original_name).into()
            }

            fn schema_id() -> ::std::borrow::Cow<'static, str> {
                let helper = <#helper_ident #ty_generics as ::schemars::JsonSchema>::schema_id()
                    .into_owned();
                helper.replace(#helper_name, #original_name).into()
            }

            fn json_schema(generator: &mut ::schemars::SchemaGenerator) -> ::schemars::Schema {
                let mut __schema =
                    <#helper_ident #ty_generics as ::schemars::JsonSchema>::json_schema(generator);

                if let Some(__props) = __schema
                    .ensure_object()
                    .get_mut("properties")
                    .and_then(|value| value.as_object_mut())
                {
                    #(#patch_stmts)*
                }

                {
                    let __root = __schema.ensure_object();
                    let __required = __root
                        .entry("required".to_owned())
                        .or_insert_with(|| ::serde_json::Value::Array(::std::vec::Vec::new()));
                    if let ::serde_json::Value::Array(__required_arr) = __required {
                        #(
                            if !__required_arr.iter().any(|v| v == #required_fields) {
                                __required_arr.push(::serde_json::Value::from(#required_fields));
                            }
                        )*
                    }
                }

                __schema
            }
        }
    })
}

fn struct_rename_all(attrs: &[Attribute]) -> Option<RenameRule> {
    rename_all_from_attr(attrs, "schemars")
        .or_else(|| rename_all_from_attr(attrs, "serde"))
}

fn rename_all_from_attr(attrs: &[Attribute], attr_name: &str) -> Option<RenameRule> {
    for attr in attrs {
        if !attr.path().is_ident(attr_name) {
            continue;
        }

        let Ok(nested) = attr.parse_args_with(
            syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
        ) else {
            continue;
        };

        for meta in &nested {
            if let syn::Meta::NameValue(nv) = meta {
                if nv.path.is_ident("rename_all") {
                    if let Some(value) = expr_to_str(&nv.value) {
                        if let Some(rule) = parse_rename_rule(&value) {
                            return Some(rule);
                        }
                    }
                }
            }
        }
    }

    None
}

fn parse_rename_rule(value: &str) -> Option<RenameRule> {
    match value {
        "lowercase" => Some(RenameRule::Lower),
        "UPPERCASE" => Some(RenameRule::Upper),
        "PascalCase" => Some(RenameRule::Pascal),
        "camelCase" => Some(RenameRule::Camel),
        "snake_case" => Some(RenameRule::Snake),
        "SCREAMING_SNAKE_CASE" => Some(RenameRule::ScreamingSnake),
        "kebab-case" => Some(RenameRule::Kebab),
        "SCREAMING-KEBAB-CASE" => Some(RenameRule::ScreamingKebab),
        _ => None,
    }
}

fn apply_rename_rule(name: &str, rule: RenameRule) -> String {
    let words = split_words(name);

    match rule {
        RenameRule::Lower => words.join("").to_lowercase(),
        RenameRule::Upper => words.join("").to_uppercase(),
        RenameRule::Pascal => words.iter().map(|w| capitalize(w)).collect::<String>(),
        RenameRule::Camel => {
            let mut iter = words.iter();
            let Some(first) = iter.next() else { return String::new(); };
            let mut out = first.to_lowercase();
            for word in iter {
                out.push_str(&capitalize(word));
            }
            out
        }
        RenameRule::Snake => words.join("_").to_lowercase(),
        RenameRule::ScreamingSnake => words.join("_").to_uppercase(),
        RenameRule::Kebab => words.join("-").to_lowercase(),
        RenameRule::ScreamingKebab => words.join("-").to_uppercase(),
    }
}

fn split_words(name: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut prev_is_lower_or_digit = false;

    for ch in name.chars() {
        if ch == '_' || ch == '-' || ch == ' ' {
            if !current.is_empty() {
                words.push(current.to_lowercase());
                current.clear();
            }
            prev_is_lower_or_digit = false;
            continue;
        }

        let is_upper = ch.is_ascii_uppercase();
        if is_upper && prev_is_lower_or_digit && !current.is_empty() {
            words.push(current.to_lowercase());
            current.clear();
        }

        current.push(ch);
        prev_is_lower_or_digit = ch.is_ascii_lowercase() || ch.is_ascii_digit();
    }

    if !current.is_empty() {
        words.push(current.to_lowercase());
    }

    words
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => {
            let mut out = first.to_ascii_uppercase().to_string();
            out.push_str(chars.as_str());
            out
        }
        None => String::new(),
    }
}

fn generate_helper_struct(
    helper_ident: &Ident,
    struct_attrs: &[Attribute],
    fields: &Fields,
    field_info: &[FieldInfo],
    generics: &syn::Generics,
) -> syn::Result<TokenStream> {
    let named = match fields {
        Fields::Named(_) => field_info,
        _ => {
            return Err(syn::Error::new_spanned(
                fields,
                "derive macro requires named fields",
            ))
        }
    };

    let helper_fields = named.iter().map(|info| {
        let attrs = &info.field.attrs;
        let ident = info.field.ident.as_ref().unwrap();
        let ty = &info.field.ty;
        let vis = &info.field.vis;
        quote! {
            #(#attrs)*
            #vis #ident: #ty
        }
    });

    let helper_generics = generics.clone();

    Ok(quote! {
        #[allow(non_camel_case_types)]
        #[derive(::schemars::JsonSchema)]
        #(#struct_attrs)*
        struct #helper_ident #helper_generics {
            #(#helper_fields),*
        }
    })
}

fn generate_constraint_stmts(c: &SchemaConstraints) -> TokenStream {
    let mut stmts = TokenStream::new();

    if let Some(min) = c.min_length {
        stmts.extend(quote! {
            let __is_array = matches!(
                __prop_obj.get("type"),
                Some(::serde_json::Value::String(kind)) if kind == "array"
            ) || matches!(
                __prop_obj.get("type"),
                Some(::serde_json::Value::Array(kinds)) if kinds.iter().any(|kind| kind == "array")
            );
            let __length_key = if __is_array { "minItems" } else { "minLength" };
            __prop_obj.insert(__length_key.to_owned(), ::serde_json::Value::from(#min));
        });
    }
    if let Some(max) = c.max_length {
        stmts.extend(quote! {
            let __is_array = matches!(
                __prop_obj.get("type"),
                Some(::serde_json::Value::String(kind)) if kind == "array"
            ) || matches!(
                __prop_obj.get("type"),
                Some(::serde_json::Value::Array(kinds)) if kinds.iter().any(|kind| kind == "array")
            );
            let __length_key = if __is_array { "maxItems" } else { "maxLength" };
            __prop_obj.insert(__length_key.to_owned(), ::serde_json::Value::from(#max));
        });
    }
    if let Some(ref min) = c.minimum {
        let v = numeric_lit_to_tokens(min);
        stmts.extend(quote! { __prop_obj.insert("minimum".to_owned(), #v); });
    }
    if let Some(ref max) = c.maximum {
        let v = numeric_lit_to_tokens(max);
        stmts.extend(quote! { __prop_obj.insert("maximum".to_owned(), #v); });
    }
    if let Some(ref fmt) = c.format {
        stmts.extend(quote! {
            __prop_obj.insert("format".to_owned(), ::serde_json::Value::from(#fmt));
        });
    }
    if let Some(ref pat) = c.pattern {
        stmts.extend(quote! {
            __prop_obj.insert("pattern".to_owned(), ::serde_json::Value::from(#pat));
        });
    }

    stmts
}

fn numeric_lit_to_tokens(n: &NumericLit) -> TokenStream {
    match n {
        NumericLit::Int(v) => quote! { ::serde_json::Value::from(#v as i64) },
        NumericLit::Float(v) => quote! { ::serde_json::Value::from(#v as f64) },
    }
}

fn parse_key_value_params<F>(tokens: &proc_macro2::TokenStream, mut handler: F)
where
    F: FnMut(&str, &Expr),
{
    let Ok(params) =
        syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated
            .parse2(tokens.clone())
    else {
        return;
    };
    for param in &params {
        if let syn::Meta::NameValue(nv) = param {
            if let Some(ident) = nv.path.get_ident() {
                handler(&ident.to_string(), &nv.value);
            }
        }
    }
}

fn expr_to_u64(expr: &Expr) -> Option<u64> {
    if let Expr::Lit(expr_lit) = expr {
        if let Lit::Int(lit_int) = &expr_lit.lit {
            return lit_int.base10_parse::<u64>().ok();
        }
    }
    None
}

fn expr_to_numeric(expr: &Expr) -> Option<NumericLit> {
    match expr {
        Expr::Lit(expr_lit) => match &expr_lit.lit {
            Lit::Int(lit_int) => lit_int.base10_parse::<i64>().ok().map(NumericLit::Int),
            Lit::Float(lit_float) => lit_float.base10_parse::<f64>().ok().map(NumericLit::Float),
            _ => None,
        },
        Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Neg(_)) => {
            match expr_to_numeric(&unary.expr) {
                Some(NumericLit::Int(n)) => Some(NumericLit::Int(-n)),
                Some(NumericLit::Float(n)) => Some(NumericLit::Float(-n)),
                None => None,
            }
        }
        _ => None,
    }
}

fn expr_to_str(expr: &Expr) -> Option<String> {
    if let Expr::Lit(expr_lit) = expr {
        if let Lit::Str(lit_str) = &expr_lit.lit {
            return Some(lit_str.value());
        }
    }
    None
}

fn regex_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        if "\\^$.|?*+()[]{}".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

fn parse_length_params(tokens: &proc_macro2::TokenStream, c: &mut SchemaConstraints) {
    parse_key_value_params(tokens, |key, expr| match key {
        "min" => {
            if let Some(n) = expr_to_u64(expr) {
                c.min_length = Some(n);
            }
        }
        "max" => {
            if let Some(n) = expr_to_u64(expr) {
                c.max_length = Some(n);
            }
        }
        "equal" => {
            if let Some(n) = expr_to_u64(expr) {
                c.min_length = Some(n);
                c.max_length = Some(n);
            }
        }
        _ => {}
    });
}

fn parse_range_params(tokens: &proc_macro2::TokenStream, c: &mut SchemaConstraints) {
    parse_key_value_params(tokens, |key, expr| match key {
        "min" => {
            if let Some(n) = expr_to_numeric(expr) {
                c.minimum = Some(n);
            }
        }
        "max" => {
            if let Some(n) = expr_to_numeric(expr) {
                c.maximum = Some(n);
            }
        }
        _ => {}
    });
}

pub(crate) fn expand_garde(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    expand_derive_with(input, "GardeSchema", "garde", parse_garde_constraints)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn parse_garde_constraints(attrs: &[Attribute]) -> SchemaConstraints {
    let mut c = SchemaConstraints::default();
    for attr in attrs {
        if !attr.path().is_ident("garde") {
            continue;
        }
        let Ok(nested) = attr.parse_args_with(
            syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
        ) else {
            continue;
        };

        for meta in &nested {
            match meta {
                syn::Meta::Path(path) => {
                    if path.is_ident("required") {
                        c.required = true;
                    } else if path.is_ident("email") {
                        c.format = Some("email".into());
                    } else if path.is_ident("url") {
                        c.format = Some("uri".into());
                    } else if path.is_ident("ip") {
                        c.format = Some("ip".into());
                    } else if path.is_ident("alphanumeric") {
                        c.pattern = Some("^[a-zA-Z0-9]*$".into());
                    }
                }
                syn::Meta::List(list) => {
                    if list.path.is_ident("length") || list.path.is_ident("byte_length") {
                        parse_length_params(&list.tokens, &mut c);
                    } else if list.path.is_ident("range") {
                        parse_range_params(&list.tokens, &mut c);
                    } else if list.path.is_ident("pattern") {
                        if let Ok(lit) = syn::parse2::<syn::LitStr>(list.tokens.clone()) {
                            c.pattern = Some(lit.value());
                        }
                    } else if list.path.is_ident("contains") {
                        if let Ok(lit) = syn::parse2::<syn::LitStr>(list.tokens.clone()) {
                            c.pattern = Some(regex_escape(&lit.value()));
                        }
                    } else if list.path.is_ident("prefix") {
                        if let Ok(lit) = syn::parse2::<syn::LitStr>(list.tokens.clone()) {
                            c.pattern = Some(format!("^{}", regex_escape(&lit.value())));
                        }
                    } else if list.path.is_ident("suffix") {
                        if let Ok(lit) = syn::parse2::<syn::LitStr>(list.tokens.clone()) {
                            c.pattern = Some(format!("{}$", regex_escape(&lit.value())));
                        }
                    }
                }
                _ => {}
            }
        }
    }
    c
}

pub(crate) fn expand_validator(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    expand_derive_with(input, "ValidatorSchema", "validate", parse_validator_constraints)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn parse_validator_constraints(attrs: &[Attribute]) -> SchemaConstraints {
    let mut c = SchemaConstraints::default();
    for attr in attrs {
        if !attr.path().is_ident("validate") {
            continue;
        }
        let Ok(nested) = attr.parse_args_with(
            syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
        ) else {
            continue;
        };

        for meta in &nested {
            match meta {
                syn::Meta::Path(path) => {
                    if path.is_ident("required") {
                        c.required = true;
                    } else if path.is_ident("email") {
                        c.format = Some("email".into());
                    } else if path.is_ident("url") {
                        c.format = Some("uri".into());
                    }
                }
                syn::Meta::List(list) => {
                    if list.path.is_ident("length") {
                        parse_length_params(&list.tokens, &mut c);
                    } else if list.path.is_ident("range") {
                        parse_range_params(&list.tokens, &mut c);
                    } else if list.path.is_ident("contains") {
                        parse_key_value_params(&list.tokens, |key, expr| {
                            if key == "pattern" {
                                if let Some(s) = expr_to_str(expr) {
                                    c.pattern = Some(regex_escape(&s));
                                }
                            }
                        });
                    } else if list.path.is_ident("email") {
                        c.format = Some("email".into());
                    } else if list.path.is_ident("url") {
                        c.format = Some("uri".into());
                    }
                }
                syn::Meta::NameValue(nv) => {
                    if let Some(ident) = nv.path.get_ident() {
                        if ident == "contains" {
                            if let Some(s) = expr_to_str(&nv.value) {
                                c.pattern = Some(regex_escape(&s));
                            }
                        }
                    }
                }
            }
        }
    }
    c
}
