//! Proc macros that derive [`cuddly::ToCddl`] and [`cuddly::CddlSpec`] for
//! minicbor wire types.
//!
//! # `#[derive(ToCddl)]`
//!
//! Reads existing minicbor annotations — no additional decoration needed on
//! the common path:
//!
//! - `#[n(N)]` on a field or variant — positional CBOR index.
//! - `#[cbor(n(N), with = "...")]` — same index, custom codec.
//! - `#[cbor(transparent)]` — delegates to the single inner field.
//!
//! Doc comments (`///`) on types, fields, and variants are carried through
//! into the generated CDDL as `;` line comments.
//!
//! Escape-hatch overrides (all under `#[cddl(...)]`, only needed for opaque
//! external types that cannot implement `ToCddl`):
//!
//! - `ty = "cddl-type"` on a field — literal CDDL type reference.
//! - `bytes` on a field — shorthand for `ty = "bytes"`.
//! - `inner = "cddl-type"` on a transparent struct — overrides the inner ref.
//!
//! # `#[derive(CddlSpec)]`
//!
//! Assembles a complete CDDL spec from a list of types. Configure with
//! `#[cddl_spec(...)]`:
//!
//! ```ignore
//! use cddl::{CddlSpec, ToCddl};
//!
//! #[derive(CddlSpec)]
//! #[cddl_spec(
//!     naming = "two",                          // "one" | "two" | "full" (default: "one")
//!     types(
//!         quote::Error,                        // naming="two" → "quote-error"
//!         sync::Response,                      // naming="two" → "sync-response"
//!         endpoints::info::NodeInfo,           // naming="two" → "info-node-info"
//!         squash::Proposal as "squash-proposal",  // explicit override
//!     )
//! )]
//! pub struct MySpec;
//!
//! fn main() {
//!     print!("{}", MySpec::cddl());
//! }
//! ```
//!
//! ## Naming strategies
//!
//! | Strategy | `quote::Error` | `endpoints::info::Response` |
//! |----------|----------------|------------------------------|
//! | `"one"`  | `error`        | `response`                   |
//! | `"two"`  | `quote-error`  | `info-response`              |
//! | `"full"` | `quote-error`  | `endpoints-info-response`    |
//!
//! The default is `"one"`. Use `"two"` when your codebase has same-named types
//! in different modules (e.g. `quote::Error` and `sync::Error`).
//!
//! Use `as "name"` to override any individual entry regardless of strategy:
//!
//! ```ignore
//! #[cddl_spec(types(
//!     squash::Proposal as "squash-proposal",
//! ))]
//! ```
//!
//! ## Comparison with utoipa
//!
//! utoipa requires `#[schema(rename = "...")]` on the struct itself to resolve
//! name collisions — the decoration lives on the type. Here, renaming happens
//! entirely at the spec registration site: the structs stay undecorated, and
//! `CddlSpec` applies the chosen strategy (or per-entry `as` override) when
//! assembling the output. The same type can appear under different names in
//! different specs without touching the type definition.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as Ts2;
use quote::quote;
use syn::{Data, DeriveInput, Fields, LitInt, Meta, parse_macro_input, punctuated::Punctuated};

// ---------------------------------------------------------------------------
// ToCddl entry point
// ---------------------------------------------------------------------------

#[proc_macro_derive(ToCddl, attributes(cddl, n, cbor))]
pub fn derive_to_cddl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand_to_cddl(input) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

// ---------------------------------------------------------------------------
// CddlSpec entry point
// ---------------------------------------------------------------------------

#[proc_macro_derive(CddlSpec, attributes(cddl_spec))]
pub fn derive_cddl_spec(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand_cddl_spec(input) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

// ---------------------------------------------------------------------------
// CddlSpec — parsed input
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum NamingStrategy {
    /// Last segment only: `quote::Error` → `error`  (default)
    One,
    /// Last two segments: `quote::Error` → `quote-error`
    Two,
    /// All segments: `endpoints::info::Response` → `endpoints-info-response`
    Full,
}

struct SpecEntry {
    path: syn::Path,
    rename: Option<String>,
}

struct SpecArgs {
    naming: NamingStrategy,
    entries: Vec<SpecEntry>,
}

// ---------------------------------------------------------------------------
// CddlSpec — expansion
// ---------------------------------------------------------------------------

fn expand_cddl_spec(input: DeriveInput) -> syn::Result<Ts2> {
    let ident = &input.ident;
    let args = parse_cddl_spec_args(&input.attrs)?;

    let entry_tokens: Vec<Ts2> = args
        .entries
        .iter()
        .map(|e| {
            let path = &e.path;
            let bare = last_segment_kebab(path);
            let cddl_name = e
                .rename
                .clone()
                .unwrap_or_else(|| compute_name(path, args.naming));
            quote! {
                if let Some(def) = <#path as ::cuddly::ToCddl>::cddl_definition() {
                    // Rename: replace the bare name ToCddl emitted with the
                    // namespaced name chosen at the spec level.
                    let def = ::cuddly::rename_rule(&def, #bare, #cddl_name);
                    let key = def
                        .split(" =")
                        .next()
                        .unwrap_or(&def)
                        .trim()
                        .to_string();
                    if seen.insert(key) {
                        out.push_str(&def);
                        out.push_str("\n\n");
                    }
                }
            }
        })
        .collect();

    Ok(quote! {
        impl ::cuddly::CddlSpec for #ident {
            fn cddl() -> String {
                let mut seen = ::std::collections::HashSet::<String>::new();
                let mut out = String::new();
                #(#entry_tokens)*
                out
            }
        }
    })
}

// ---------------------------------------------------------------------------
// CddlSpec — attribute parsing
// ---------------------------------------------------------------------------

fn parse_cddl_spec_args(attrs: &[syn::Attribute]) -> syn::Result<SpecArgs> {
    let mut naming = NamingStrategy::One;
    let mut entries: Vec<SpecEntry> = vec![];

    for attr in attrs {
        if !attr.path().is_ident("cddl_spec") {
            continue;
        }
        let Meta::List(ml) = &attr.meta else { continue };

        let nested = ml.parse_args_with(Punctuated::<Meta, syn::Token![,]>::parse_terminated)?;

        for meta in &nested {
            match meta {
                Meta::NameValue(nv) if nv.path.is_ident("naming") => {
                    if let syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(s),
                        ..
                    }) = &nv.value
                    {
                        naming = match s.value().as_str() {
                            "one" => NamingStrategy::One,
                            "two" => NamingStrategy::Two,
                            "full" => NamingStrategy::Full,
                            other => {
                                return Err(syn::Error::new_spanned(
                                    s,
                                    format!(
                                        "unknown naming strategy {:?}; \
                                     expected \"one\", \"two\", or \"full\"",
                                        other
                                    ),
                                ));
                            }
                        };
                    }
                }
                Meta::List(inner) if inner.path.is_ident("types") => {
                    entries = parse_type_entries(inner)?;
                }
                _ => {}
            }
        }
    }

    Ok(SpecArgs { naming, entries })
}

/// Parse the token stream inside `types(...)`.
///
/// Each entry is a path optionally followed by `as "literal"`:
///   `quote::Error as "quote-error"`
fn parse_type_entries(ml: &syn::MetaList) -> syn::Result<Vec<SpecEntry>> {
    use syn::parse::Parser;

    let parser = |input: syn::parse::ParseStream| -> syn::Result<Vec<SpecEntry>> {
        let mut entries = vec![];
        while !input.is_empty() {
            let path: syn::Path = input.parse()?;
            let rename = if input.peek(syn::Token![as]) {
                let _: syn::Token![as] = input.parse()?;
                let lit: syn::LitStr = input.parse()?;
                Some(lit.value())
            } else {
                None
            };
            entries.push(SpecEntry { path, rename });
            if input.peek(syn::Token![,]) {
                let _: syn::Token![,] = input.parse()?;
            }
        }
        Ok(entries)
    };

    parser.parse2(ml.tokens.clone())
}

// ---------------------------------------------------------------------------
// Naming helpers
// ---------------------------------------------------------------------------

fn last_segment_kebab(path: &syn::Path) -> String {
    to_kebab_case(
        &path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default(),
    )
}

fn compute_name(path: &syn::Path, strategy: NamingStrategy) -> String {
    let segs: Vec<String> = path
        .segments
        .iter()
        .map(|s| to_kebab_case(&s.ident.to_string()))
        .collect();

    match strategy {
        NamingStrategy::One => segs.last().cloned().unwrap_or_default(),
        NamingStrategy::Two => {
            let n = segs.len();
            if n >= 2 {
                segs[n - 2..].join("-")
            } else {
                segs.last().cloned().unwrap_or_default()
            }
        }
        NamingStrategy::Full => segs.join("-"),
    }
}

// ---------------------------------------------------------------------------
// ToCddl expansion
// ---------------------------------------------------------------------------

fn expand_to_cddl(input: DeriveInput) -> syn::Result<Ts2> {
    let ident = &input.ident;
    // Always emit the bare kebab-case type name. CddlSpec renames at assembly.
    let cddl_name = to_kebab_case(&ident.to_string());

    let transparent = is_cbor_transparent(&input.attrs);
    let inner_override = cddl_str_attr(&input.attrs, "inner");
    let type_doc = extract_doc(&input.attrs);

    let (ref_impl, def_impl) = match &input.data {
        Data::Struct(s) => expand_struct(
            &cddl_name,
            &s.fields,
            transparent,
            inner_override,
            &type_doc,
        )?,
        Data::Enum(e) => expand_enum(&cddl_name, &e.variants, &type_doc)?,
        Data::Union(_) => {
            return Err(syn::Error::new_spanned(
                ident,
                "ToCddl cannot be derived for unions",
            ));
        }
    };

    Ok(quote! {
        impl ::cuddly::ToCddl for #ident {
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
    type_doc: &[String],
) -> syn::Result<(Ts2, Ts2)> {
    let ref_impl = quote! { #cddl_name.to_string() };

    if transparent {
        let header = doc_comment_lines(type_doc);
        let def_impl = if let Some(inner_name) = inner_override {
            quote! { Some(format!("{}{} = {}", #header, #cddl_name, #inner_name)) }
        } else {
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
                Some(format!(
                    "{}{} = {}",
                    #header, #cddl_name,
                    <#inner_ty as ::cuddly::ToCddl>::cddl_ref()
                ))
            }
        };
        return Ok((ref_impl, def_impl));
    }

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

    let header = doc_comment_lines(type_doc);
    let mut fmt = format!("{}{} = [", header, cddl_name);
    let mut args: Vec<Ts2> = vec![];

    for (_, field) in &indexed {
        let label = field
            .ident
            .as_ref()
            .map(|i| i.to_string())
            .unwrap_or_default();
        let field_doc = extract_doc(&field.attrs);
        let comment = field_inline_comment(&label, &field_doc);
        fmt.push_str(&format!("\n  {{}},{}", comment));
        args.push(field_ref_expr(field));
    }
    fmt.push_str("\n]");

    Ok((ref_impl, quote! { Some(format!(#fmt, #(#args),*)) }))
}

// ---------------------------------------------------------------------------
// Enum expansion
// ---------------------------------------------------------------------------

fn expand_enum(
    cddl_name: &str,
    variants: &Punctuated<syn::Variant, syn::token::Comma>,
    type_doc: &[String],
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

    let header = doc_comment_lines(type_doc);
    let mut fmt = String::new();
    let mut args: Vec<Ts2> = vec![];

    for (i, (n_val, variant)) in indexed.iter().enumerate() {
        let sep = if i == 0 { "" } else { "\n/ " };
        let variant_name = variant.ident.to_string();
        let variant_doc = extract_doc(&variant.attrs);
        let comment = field_inline_comment(&variant_name, &variant_doc);

        match &variant.fields {
            Fields::Unit => {
                fmt.push_str(&format!("{}{}{}", sep, n_val, comment));
            }
            Fields::Unnamed(u) => {
                let mut inner: Vec<(u64, &syn::Field)> = u
                    .unnamed
                    .iter()
                    .filter_map(|f| n_index(&f.attrs).map(|n| (n, f)))
                    .collect();
                inner.sort_by_key(|(n, _)| *n);
                if inner.is_empty() {
                    fmt.push_str(&format!("{}{}{}", sep, n_val, comment));
                } else {
                    let fmts: Vec<String> = inner.iter().map(|_| "{}".to_string()).collect();
                    let fargs: Vec<Ts2> = inner.iter().map(|(_, f)| field_ref_expr(f)).collect();
                    fmt.push_str(&format!(
                        "{}[{}, {}]{}",
                        sep,
                        n_val,
                        fmts.join(", "),
                        comment
                    ));
                    args.extend(fargs);
                }
            }
            Fields::Named(n) => {
                let mut inner: Vec<(u64, &syn::Field)> = n
                    .named
                    .iter()
                    .filter_map(|f| n_index(&f.attrs).map(|idx| (idx, f)))
                    .collect();
                inner.sort_by_key(|(idx, _)| *idx);
                let fmts: Vec<String> = inner.iter().map(|_| "{}".to_string()).collect();
                let fargs: Vec<Ts2> = inner.iter().map(|(_, f)| field_ref_expr(f)).collect();
                fmt.push_str(&format!(
                    "{}[{}, {}]{}",
                    sep,
                    n_val,
                    fmts.join(", "),
                    comment
                ));
                args.extend(fargs);
            }
        }
    }

    let full_fmt = format!("{}{} = {}", header, cddl_name, fmt);
    let def_impl = if args.is_empty() {
        quote! { Some(#full_fmt.to_string()) }
    } else {
        quote! { Some(format!(#full_fmt, #(#args),*)) }
    };

    Ok((ref_impl, def_impl))
}

// ---------------------------------------------------------------------------
// Doc comment extraction
// ---------------------------------------------------------------------------

fn extract_doc(attrs: &[syn::Attribute]) -> Vec<String> {
    attrs
        .iter()
        .filter_map(|a| {
            if !a.path().is_ident("doc") {
                return None;
            }
            if let Meta::NameValue(nv) = &a.meta {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) = &nv.value
                {
                    let line = s.value();
                    return Some(line.trim().to_string());
                }
            }
            None
        })
        .filter(|s| !s.is_empty())
        .collect()
}

fn doc_comment_lines(lines: &[String]) -> String {
    if lines.is_empty() {
        return String::new();
    }
    lines.iter().map(|l| format!("; {}\n", l)).collect()
}

fn field_inline_comment(name: &str, doc: &[String]) -> String {
    if doc.is_empty() {
        format!("  ; {}", name)
    } else {
        format!("  ; {}: {}", name, doc.join(" "))
    }
}

// ---------------------------------------------------------------------------
// Attribute helpers
// ---------------------------------------------------------------------------

fn n_index(attrs: &[syn::Attribute]) -> Option<u64> {
    for attr in attrs {
        if attr.path().is_ident("n")
            && let Meta::List(ml) = &attr.meta
            && let Ok(lit) = ml.parse_args::<LitInt>()
        {
            return lit.base10_parse().ok();
        }
        if attr.path().is_ident("cbor")
            && let Meta::List(ml) = &attr.meta
        {
            let nested = ml.parse_args_with(Punctuated::<Meta, syn::Token![,]>::parse_terminated);
            if let Ok(nested) = nested {
                for meta in &nested {
                    if let Meta::List(inner) = meta
                        && inner.path.is_ident("n")
                        && let Ok(lit) = inner.parse_args::<LitInt>()
                    {
                        return lit.base10_parse().ok();
                    }
                }
            }
        }
    }
    None
}

fn cddl_type_attr(attrs: &[syn::Attribute]) -> Option<String> {
    if cddl_flag_attr(attrs, "bytes") {
        return Some("bytes".to_string());
    }
    cddl_str_attr(attrs, "ty")
}

fn cddl_str_attr(attrs: &[syn::Attribute], key: &str) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident("cddl")
            && let Meta::List(ml) = &attr.meta
        {
            let nested = ml.parse_args_with(Punctuated::<Meta, syn::Token![,]>::parse_terminated);
            if let Ok(nested) = nested {
                for meta in &nested {
                    if let Meta::NameValue(nv) = meta
                        && nv.path.is_ident(key)
                        && let syn::Expr::Lit(syn::ExprLit {
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
    None
}

fn cddl_flag_attr(attrs: &[syn::Attribute], flag: &str) -> bool {
    for attr in attrs {
        if attr.path().is_ident("cddl")
            && let Meta::List(ml) = &attr.meta
        {
            let nested = ml.parse_args_with(Punctuated::<Meta, syn::Token![,]>::parse_terminated);
            if let Ok(nested) = nested {
                for meta in &nested {
                    if let Meta::Path(p) = meta
                        && p.is_ident(flag)
                    {
                        return true;
                    }
                }
            }
        }
    }
    false
}

fn is_cbor_transparent(attrs: &[syn::Attribute]) -> bool {
    for attr in attrs {
        if attr.path().is_ident("cbor")
            && let Meta::List(ml) = &attr.meta
        {
            let nested = ml.parse_args_with(Punctuated::<Meta, syn::Token![,]>::parse_terminated);
            if let Ok(nested) = nested {
                for meta in &nested {
                    if let Meta::Path(p) = meta
                        && p.is_ident("transparent")
                    {
                        return true;
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

/// Priority:
/// 1. `#[cddl(ty = "...")]` or `#[cddl(bytes)]` — explicit override
/// 2. `#[cbor(with = "some::module")]` — look up `some::module::CDDL`
/// 3. Fall through to `<FieldType as ::cuddly::ToCddl>::cddl_ref()`
fn field_ref_expr(field: &syn::Field) -> Ts2 {
    // 1. explicit cddl override
    if let Some(override_name) = cddl_type_attr(&field.attrs) {
        return quote! { #override_name.to_string() };
    }

    // 2. #[cbor(with = "path::to::module")] → path::to::module::CDDL
    if let Some(with_path) = cbor_with_attr(&field.attrs) {
        // Parse the string as a syn::Path so we get proper token hygiene.
        if let Ok(path) = syn::parse_str::<syn::Path>(&with_path) {
            return quote! { #path::CDDL.to_string() };
        }
    }

    // 3. trait dispatch
    let ty = &field.ty;
    quote! { <#ty as ::cuddly::ToCddl>::cddl_ref() }
}

/// Extract the string value of `#[cbor(with = "...")]`.
fn cbor_with_attr(attrs: &[syn::Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident("cbor")
            && let Meta::List(ml) = &attr.meta
        {
            let nested = ml.parse_args_with(Punctuated::<Meta, syn::Token![,]>::parse_terminated);
            if let Ok(nested) = nested {
                for meta in &nested {
                    if let Meta::NameValue(nv) = meta
                        && nv.path.is_ident("with")
                        && let syn::Expr::Lit(syn::ExprLit {
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
    None
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

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
