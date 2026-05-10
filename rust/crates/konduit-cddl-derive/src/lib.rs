//! Proc macro that derives [`konduit_cddl::ToCddl`] for konduit wire types.
//!
//! The macro understands the minicbor attribute conventions already on every
//! wire type, so no additional annotations are needed on the common path:
//!
//! - `#[n(N)]` on a field or variant — positional CBOR index.
//! - `#[cbor(n(N), with = "...")]` — same index, custom codec.
//! - `#[cbor(transparent)]` on the type — delegates to the single inner field.
//!
//! Override attributes (all under `#[cddl(...)]`):
//! - `name = "my-name"` on the type — explicit CDDL rule name instead of
//!   the default kebab-case conversion of the Rust type name.
//! - `type = "other-type"` on a field — use this literal string as the CDDL
//!   type reference for that field (needed when the field type doesn't
//!   implement `ToCddl`, e.g. opaque external types).
//! - `bytes` on a field — shorthand for `type = "bytes"`.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as Ts2;
use quote::quote;
use syn::{
    Data, DeriveInput, Fields, LitInt, LitStr, Meta, parse_macro_input, punctuated::Punctuated,
};

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[proc_macro_derive(ToCddl, attributes(cddl, n, cbor))]
pub fn derive_to_cddl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand(input) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

// ---------------------------------------------------------------------------
// Top-level dispatch
// ---------------------------------------------------------------------------

fn expand(input: DeriveInput) -> syn::Result<Ts2> {
    let ident = &input.ident;
    let cddl_name =
        cddl_name_attr(&input.attrs).unwrap_or_else(|| to_kebab_case(&ident.to_string()));

    // Check for #[cbor(transparent)]
    let transparent = is_cbor_transparent(&input.attrs);
    // #[cddl(inner = "...")] — overrides the inner-type name for transparent structs
    let inner_override = cddl_str_attr(&input.attrs, "inner");

    let (ref_impl, def_impl) = match &input.data {
        Data::Struct(s) => expand_struct(&cddl_name, &s.fields, transparent, inner_override)?,
        Data::Enum(e) => expand_enum(&cddl_name, &e.variants)?,
        Data::Union(_) => {
            return Err(syn::Error::new_spanned(
                ident,
                "ToCddl cannot be derived for unions",
            ));
        }
    };

    Ok(quote! {
        impl ::konduit_cddl::ToCddl for #ident {
            fn cddl_ref() -> String { #ref_impl }
            fn cddl_definition() -> Option<String> { #def_impl }
        }
    })
}

// ---------------------------------------------------------------------------
// Struct expansion
// ---------------------------------------------------------------------------

fn expand_struct(
    cddl_name: &str,
    fields: &Fields,
    transparent: bool,
    inner_override: Option<String>,
) -> syn::Result<(Ts2, Ts2)> {
    let ref_impl = quote! { #cddl_name.to_string() };

    if transparent {
        let def_impl = if let Some(inner_name) = inner_override {
            // #[cddl(inner = "type-name")] — explicit override, avoids requiring
            // the inner type to implement ToCddl (useful for external/opaque types).
            quote! {
                Some(format!("{} = {}", #cddl_name, #inner_name))
            }
        } else {
            // Derive from the single inner field's Rust type.
            let inner_ty = match fields {
                Fields::Named(n) if n.named.len() == 1 => n.named.first().unwrap().ty.clone(),
                Fields::Unnamed(u) if u.unnamed.len() == 1 => u.unnamed.first().unwrap().ty.clone(),
                _ => {
                    return Err(syn::Error::new(
                        proc_macro2::Span::call_site(),
                        "#[cbor(transparent)] requires exactly one field",
                    ));
                }
            };
            quote! {
                Some(format!("{} = {}", #cddl_name, <#inner_ty as ::konduit_cddl::ToCddl>::cddl_ref()))
            }
        };
        return Ok((ref_impl, def_impl));
    }

    // Collect fields indexed by their #[n(N)] value.
    let raw_fields: Vec<&syn::Field> = match fields {
        Fields::Named(n) => n.named.iter().collect(),
        Fields::Unnamed(u) => u.unnamed.iter().collect(),
        Fields::Unit => vec![],
    };

    let mut indexed: Vec<(u64, &syn::Field)> = raw_fields
        .iter()
        .filter_map(|f| n_index(&f.attrs).map(|n| (n, *f)))
        .collect();
    indexed.sort_by_key(|(n, _)| *n);

    if indexed.is_empty() {
        return Ok((ref_impl, quote! { None }));
    }

    // Build format string and arguments for the definition.
    // Pattern: "name = [\n  {ref0},  ; field0\n  {ref1},  ; field1\n]"
    let mut fmt = format!("{} = [", cddl_name);
    let mut args: Vec<Ts2> = vec![];

    for (_, field) in &indexed {
        let label = field
            .ident
            .as_ref()
            .map(|i| i.to_string())
            .unwrap_or_default();
        fmt.push_str(&format!("\n  {{}},  ; {}", label));
        args.push(field_ref_expr(field));
    }
    fmt.push_str("\n]");

    let def_impl = quote! {
        Some(format!(#fmt, #(#args),*))
    };

    Ok((ref_impl, def_impl))
}

// ---------------------------------------------------------------------------
// Enum expansion
// ---------------------------------------------------------------------------

fn expand_enum(
    cddl_name: &str,
    variants: &Punctuated<syn::Variant, syn::token::Comma>,
) -> syn::Result<(Ts2, Ts2)> {
    let ref_impl = quote! { #cddl_name.to_string() };

    let mut indexed: Vec<(u64, &syn::Variant)> = variants
        .iter()
        .filter_map(|v| n_index(&v.attrs).map(|n| (n, v)))
        .collect();
    indexed.sort_by_key(|(n, _)| *n);

    if indexed.is_empty() {
        return Ok((ref_impl, quote! { None }));
    }

    // Build choice expression: each variant is one alternative.
    // Unit variants → bare integer (minicbor encodes as CBOR uint).
    // Newtype / struct variants → [n, field0, field1, ...]
    let mut fmt = String::new();
    let mut args: Vec<Ts2> = vec![];

    for (i, (n_val, variant)) in indexed.iter().enumerate() {
        let sep = if i == 0 { "" } else { "\n/ " };
        let variant_name = &variant.ident.to_string();

        match &variant.fields {
            Fields::Unit => {
                fmt.push_str(&format!("{}{}", sep, n_val));
                // no args for unit
                // Append comment after the integer
                fmt.push_str(&format!("  ; {}", variant_name));
            }
            Fields::Unnamed(u) => {
                // Collect inner fields by their #[n(N)], sorted.
                let mut inner: Vec<(u64, &syn::Field)> = u
                    .unnamed
                    .iter()
                    .filter_map(|f| n_index(&f.attrs).map(|n| (n, f)))
                    .collect();
                inner.sort_by_key(|(n, _)| *n);

                let field_fmts: Vec<String> = inner.iter().map(|_| "{}".to_string()).collect();
                let field_args: Vec<Ts2> = inner.iter().map(|(_, f)| field_ref_expr(f)).collect();

                if field_fmts.is_empty() {
                    // Newtype with no indexed field — treat as unit
                    fmt.push_str(&format!("{}{}", sep, n_val));
                    fmt.push_str(&format!("  ; {}", variant_name));
                } else {
                    fmt.push_str(&format!(
                        "{}[{}, {}]  ; {}",
                        sep,
                        n_val,
                        field_fmts.join(", "),
                        variant_name,
                    ));
                    args.extend(field_args);
                }
            }
            Fields::Named(n) => {
                let mut inner: Vec<(u64, &syn::Field)> = n
                    .named
                    .iter()
                    .filter_map(|f| n_index(&f.attrs).map(|idx| (idx, f)))
                    .collect();
                inner.sort_by_key(|(idx, _)| *idx);

                let field_fmts: Vec<String> = inner.iter().map(|_| "{}".to_string()).collect();
                let field_args: Vec<Ts2> = inner.iter().map(|(_, f)| field_ref_expr(f)).collect();

                fmt.push_str(&format!(
                    "{}[{}, {}]  ; {}",
                    sep,
                    n_val,
                    field_fmts.join(", "),
                    variant_name,
                ));
                args.extend(field_args);
            }
        }
    }

    let full_fmt = format!("{} = {}", cddl_name, fmt);

    let def_impl = if args.is_empty() {
        quote! { Some(#full_fmt.to_string()) }
    } else {
        quote! { Some(format!(#full_fmt, #(#args),*)) }
    };

    Ok((ref_impl, def_impl))
}

// ---------------------------------------------------------------------------
// Attribute helpers
// ---------------------------------------------------------------------------

/// Extract the integer from `#[n(N)]` or `#[cbor(n(N), ...)]`.
fn n_index(attrs: &[syn::Attribute]) -> Option<u64> {
    for attr in attrs {
        if attr.path().is_ident("n") {
            if let Meta::List(ml) = &attr.meta {
                if let Ok(lit) = ml.parse_args::<LitInt>() {
                    return lit.base10_parse().ok();
                }
            }
        }
        if attr.path().is_ident("cbor") {
            if let Meta::List(ml) = &attr.meta {
                let nested =
                    ml.parse_args_with(Punctuated::<Meta, syn::Token![,]>::parse_terminated);
                if let Ok(nested) = nested {
                    for meta in &nested {
                        if let Meta::List(inner) = meta {
                            if inner.path.is_ident("n") {
                                if let Ok(lit) = inner.parse_args::<LitInt>() {
                                    return lit.base10_parse().ok();
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

/// Read `#[cddl(name = "...")]` from the type-level attributes.
fn cddl_name_attr(attrs: &[syn::Attribute]) -> Option<String> {
    cddl_str_attr(attrs, "name")
}

/// Read `#[cddl(type = "...")]` from a field's attributes.
fn cddl_type_attr(attrs: &[syn::Attribute]) -> Option<String> {
    // #[cddl(bytes)] shorthand
    if cddl_flag_attr(attrs, "bytes") {
        return Some("bytes".to_string());
    }
    cddl_str_attr(attrs, "ty")
}

fn cddl_str_attr(attrs: &[syn::Attribute], key: &str) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident("cddl") {
            if let Meta::List(ml) = &attr.meta {
                let nested =
                    ml.parse_args_with(Punctuated::<Meta, syn::Token![,]>::parse_terminated);
                if let Ok(nested) = nested {
                    for meta in &nested {
                        if let Meta::NameValue(nv) = meta {
                            if nv.path.is_ident(key) {
                                if let syn::Expr::Lit(syn::ExprLit {
                                    lit: syn::Lit::Str(s),
                                    ..
                                }) = &nv.value
                                {
                                    return Some(s.value());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

fn cddl_flag_attr(attrs: &[syn::Attribute], flag: &str) -> bool {
    for attr in attrs {
        if attr.path().is_ident("cddl") {
            if let Meta::List(ml) = &attr.meta {
                let nested =
                    ml.parse_args_with(Punctuated::<Meta, syn::Token![,]>::parse_terminated);
                if let Ok(nested) = nested {
                    for meta in &nested {
                        if let Meta::Path(p) = meta {
                            if p.is_ident(flag) {
                                return true;
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

/// Detect `#[cbor(transparent)]` on the type.
fn is_cbor_transparent(attrs: &[syn::Attribute]) -> bool {
    for attr in attrs {
        if attr.path().is_ident("cbor") {
            if let Meta::List(ml) = &attr.meta {
                let nested =
                    ml.parse_args_with(Punctuated::<Meta, syn::Token![,]>::parse_terminated);
                if let Ok(nested) = nested {
                    for meta in &nested {
                        if let Meta::Path(p) = meta {
                            if p.is_ident("transparent") {
                                return true;
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Field CDDL ref expression
// ---------------------------------------------------------------------------

/// Emit an expression that evaluates to the CDDL type reference for a field.
/// Respects `#[cddl(type = "...")]` and `#[cddl(bytes)]` overrides.
fn field_ref_expr(field: &syn::Field) -> Ts2 {
    if let Some(override_name) = cddl_type_attr(&field.attrs) {
        return quote! { #override_name.to_string() };
    }
    let ty = &field.ty;
    quote! { <#ty as ::konduit_cddl::ToCddl>::cddl_ref() }
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

/// Convert `PascalCase` → `kebab-case`.
/// `BackingView` → `backing-view`, `DepthBucket` → `depth-bucket`.
fn to_kebab_case(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    for (i, c) in s.char_indices() {
        if c.is_uppercase() && i > 0 {
            out.push('-');
        }
        out.extend(c.to_lowercase());
    }
    out
}
