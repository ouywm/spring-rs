//! `#[derive(GardeSchema)]` and `#[derive(ValidatorSchema)]` — auto-generate
//! `JsonSchema` impl with OpenAPI constraint injection from validation attributes.
//!
//! ## Supported mappings
//!
//! ### Garde → JSON Schema
//!
//! | Garde attribute | JSON Schema keyword |
//! |---|---|
//! | `length(min/max/equal)` | `minLength` / `maxLength` |
//! | `range(min/max)` | `minimum` / `maximum` |
//! | `email` | `format: "email"` |
//! | `url` | `format: "uri"` |
//! | `ip` | `format: "ip"` |
//! | `alphanumeric` | `pattern: "^[a-zA-Z0-9]*$"` |
//! | `pattern("regex")` | `pattern` |
//! | `contains/prefix/suffix` | `pattern` |
//!
//! ### Validator → JSON Schema
//!
//! | Validator attribute | JSON Schema keyword |
//! |---|---|
//! | `length(min/max/equal)` | `minLength` / `maxLength` |
//! | `range(min/max)` | `minimum` / `maximum` |
//! | `email` | `format: "email"` |
//! | `url` | `format: "uri"` |
//! | `contains(pattern)` | `pattern` |
//!
//! Runtime expressions are silently skipped (not determinable at compile time).

use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::parse::Parser;
use syn::{DeriveInput, Expr, Fields, Lit, Type};


#[derive(Default)]
struct SchemaConstraints {
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
    }
}

struct FieldInfo {
    name: String,
    ty: Type,
    is_option: bool,
    constraints: SchemaConstraints,
}

fn collect_field_info<F>(fields: &Fields, parse_constraints: F) -> syn::Result<Vec<FieldInfo>>
where
    F: Fn(&[syn::Attribute]) -> SchemaConstraints,
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
        .map(|f| {
            let name = f.ident.as_ref().unwrap().to_string();
            let ty = f.ty.clone();
            let is_option = is_option_type(&f.ty);
            let constraints = parse_constraints(&f.attrs);
            FieldInfo { name, ty, is_option, constraints }
        })
        .collect())
}

fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Option";
        }
    }
    false
}

fn expand_derive_with<F>(input: DeriveInput, macro_name: &str, parser: F) -> syn::Result<TokenStream>
where
    F: Fn(&[syn::Attribute]) -> SchemaConstraints,
{
    let struct_ident = &input.ident;
    let fields = match &input.data {
        syn::Data::Struct(data) => &data.fields,
        _ => {
            return Err(syn::Error::new_spanned(
                &input.ident,
                format!("{macro_name} can only be derived for structs"),
            ))
        }
    };
    let field_info = collect_field_info(fields, parser)?;
    Ok(generate_json_schema(struct_ident, &field_info))
}

fn generate_json_schema(struct_ident: &syn::Ident, fields: &[FieldInfo]) -> TokenStream {
    let struct_name_str = struct_ident.to_string();

    let required_fields: Vec<&str> = fields
        .iter()
        .filter(|f| !f.is_option)
        .map(|f| f.name.as_str())
        .collect();

    let property_names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();

    let mut schema_var_idents = Vec::new();
    let mut schema_var_exprs = Vec::new();

    for (i, field) in fields.iter().enumerate() {
        let var_ident = Ident::new(&format!("__prop_{i}"), Span::call_site());
        let ty = &field.ty;

        let expr = if field.constraints.is_empty() {
            quote! { generator.subschema_for::<#ty>() }
        } else {
            let stmts = generate_constraint_stmts(&field.constraints);
            quote! {
                {
                    let mut __s = generator.subschema_for::<#ty>();
                    let __obj = __s.ensure_object();
                    #stmts
                    __s
                }
            }
        };

        schema_var_idents.push(var_ident);
        schema_var_exprs.push(expr);
    }

    quote! {
        impl ::schemars::JsonSchema for #struct_ident {
            fn schema_name() -> ::std::borrow::Cow<'static, str> {
                #struct_name_str.into()
            }

            fn json_schema(generator: &mut ::schemars::SchemaGenerator) -> ::schemars::Schema {
                use ::schemars::json_schema;

                #(let #schema_var_idents = #schema_var_exprs;)*

                json_schema!({
                    "type": "object",
                    "required": [#(#required_fields),*],
                    "properties": {
                        #(#property_names: #schema_var_idents),*
                    }
                })
            }
        }
    }
}

fn generate_constraint_stmts(c: &SchemaConstraints) -> TokenStream {
    let mut stmts = TokenStream::new();

    if let Some(min) = c.min_length {
        stmts.extend(quote! {
            __obj.insert("minLength".to_owned(), ::serde_json::Value::from(#min));
        });
    }
    if let Some(max) = c.max_length {
        stmts.extend(quote! {
            __obj.insert("maxLength".to_owned(), ::serde_json::Value::from(#max));
        });
    }
    if let Some(ref min) = c.minimum {
        let v = numeric_lit_to_tokens(min);
        stmts.extend(quote! { __obj.insert("minimum".to_owned(), #v); });
    }
    if let Some(ref max) = c.maximum {
        let v = numeric_lit_to_tokens(max);
        stmts.extend(quote! { __obj.insert("maximum".to_owned(), #v); });
    }
    if let Some(ref fmt) = c.format {
        stmts.extend(quote! {
            __obj.insert("format".to_owned(), ::serde_json::Value::from(#fmt));
        });
    }
    if let Some(ref pat) = c.pattern {
        stmts.extend(quote! {
            __obj.insert("pattern".to_owned(), ::serde_json::Value::from(#pat));
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


/// Parse `length(min/max/equal)` and `range(min/max)` params — shared by both garde and validator.
fn parse_length_params(tokens: &proc_macro2::TokenStream, c: &mut SchemaConstraints) {
    parse_key_value_params(tokens, |key, expr| match key {
        "min" => { if let Some(n) = expr_to_u64(expr) { c.min_length = Some(n); } }
        "max" => { if let Some(n) = expr_to_u64(expr) { c.max_length = Some(n); } }
        "equal" => { if let Some(n) = expr_to_u64(expr) { c.min_length = Some(n); c.max_length = Some(n); } }
        _ => {}
    });
}

fn parse_range_params(tokens: &proc_macro2::TokenStream, c: &mut SchemaConstraints) {
    parse_key_value_params(tokens, |key, expr| match key {
        "min" => { if let Some(n) = expr_to_numeric(expr) { c.minimum = Some(n); } }
        "max" => { if let Some(n) = expr_to_numeric(expr) { c.maximum = Some(n); } }
        _ => {}
    });
}


pub(crate) fn expand_garde(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    expand_derive_with(input, "GardeSchema", parse_garde_constraints)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn parse_garde_constraints(attrs: &[syn::Attribute]) -> SchemaConstraints {
    let mut c = SchemaConstraints::default();
    for attr in attrs {
        if !attr.path().is_ident("garde") { continue; }
        let Ok(nested) = attr.parse_args_with(
            syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
        ) else { continue; };

        for meta in &nested {
            match meta {
                syn::Meta::Path(path) => {
                    if path.is_ident("email") { c.format = Some("email".into()); }
                    else if path.is_ident("url") { c.format = Some("uri".into()); }
                    else if path.is_ident("ip") { c.format = Some("ip".into()); }
                    else if path.is_ident("alphanumeric") { c.pattern = Some("^[a-zA-Z0-9]*$".into()); }
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
    expand_derive_with(input, "ValidatorSchema", parse_validator_constraints)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn parse_validator_constraints(attrs: &[syn::Attribute]) -> SchemaConstraints {
    let mut c = SchemaConstraints::default();
    for attr in attrs {
        if !attr.path().is_ident("validate") { continue; }
        let Ok(nested) = attr.parse_args_with(
            syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
        ) else { continue; };

        for meta in &nested {
            match meta {
                syn::Meta::Path(path) => {
                    if path.is_ident("email") { c.format = Some("email".into()); }
                    else if path.is_ident("url") { c.format = Some("uri".into()); }
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
