use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    Data, DeriveInput, Error, Expr, ExprLit, Fields, Lit, Meta, Variant, parse_macro_input,
    punctuated::Punctuated, token::Comma,
};

/// Derive `ProblemDetail` for an enum.
///
/// Each variant is either a leaf with full attributes, or a delegate to an
/// inner type that itself implements `ProblemDetail`.
///
/// **Leaf variant:**
/// ```rust,ignore
/// #[problem(slug = "unauthorized", title = "Unauthorized", http_status = 401)]
/// Unauthorized,
/// ```
///
/// **Delegate variant** — must be a newtype (single unnamed field):
/// ```rust,ignore
/// #[problem(delegate)]
/// Common(common::Error),
/// ```
/// All trait methods delegate to the inner value.
///
/// Set `PROBLEM_DETAIL_BASE_URL` in `.cargo/config.toml`:
/// ```toml
/// [env]
/// PROBLEM_DETAIL_BASE_URL = "https://example.com/errors"
/// ```
#[proc_macro_derive(ProblemDetail, attributes(problem))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

// ── Variant kinds ─────────────────────────────────────────────────────────────

enum VariantKind {
    Leaf {
        slug: String,
        title: String,
        http_status: u16,
        doc: String,
    },
    Delegate,
}

struct VariantMeta {
    ident: syn::Ident,
    fields: Fields,
    kind: VariantKind,
}

// ── Expansion ─────────────────────────────────────────────────────────────────

fn expand(input: DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let variants = match &input.data {
        Data::Enum(e) => &e.variants,
        _ => {
            return Err(Error::new_spanned(
                &input,
                "ProblemDetail can only be derived for enums",
            ));
        }
    };

    let metas: Vec<VariantMeta> = variants
        .iter()
        .map(parse_variant)
        .collect::<syn::Result<_>>()?;

    let slug_arms = method_arms(&metas, &|m| match &m.kind {
        VariantKind::Leaf { slug, .. } => quote!(#slug),
        VariantKind::Delegate => delegate_call(m, quote!(slug())),
    });
    let type_arms = method_arms(&metas, &|m| match &m.kind {
        VariantKind::Leaf { slug, .. } => quote!(::core::concat!(
            env!("PROBLEM_DETAIL_BASE_URL"), "/",
            env!("CARGO_PKG_NAME"), "/",
            env!("CARGO_PKG_VERSION"), "/",
            #slug
        )),
        VariantKind::Delegate => delegate_call(m, quote!(problem_type())),
    });
    let title_arms = method_arms(&metas, &|m| match &m.kind {
        VariantKind::Leaf { title, .. } => quote!(#title),
        VariantKind::Delegate => delegate_call(m, quote!(title())),
    });
    let http_status_arms = method_arms(&metas, &|m| match &m.kind {
        VariantKind::Leaf { http_status, .. } => quote!(#http_status),
        VariantKind::Delegate => delegate_call(m, quote!(http_status())),
    });

    let leaf_metas: Vec<&VariantMeta> = metas
        .iter()
        .filter(|m| matches!(m.kind, VariantKind::Leaf { .. }))
        .collect();
    let manifest = manifest_json(&leaf_metas);

    Ok(quote! {
        impl #impl_generics ::problem_details_wire::ProblemDetail for #name #ty_generics #where_clause {
            fn slug(&self)         -> &'static str { match self { #(#slug_arms)*        } }
            fn problem_type(&self) -> &'static str { match self { #(#type_arms)*        } }
            fn title(&self)        -> &'static str { match self { #(#title_arms)*       } }
            fn http_status(&self)  -> u16          { match self { #(#http_status_arms)* } }
        }

        impl #impl_generics #name #ty_generics #where_clause {
            #[doc(hidden)]
            pub const PROBLEM_DETAILS_MANIFEST: &'static str = #manifest;
            #[doc(hidden)]
            pub const PROBLEM_DETAILS_CRATE_NAME: &'static str = env!("CARGO_PKG_NAME");
            #[doc(hidden)]
            pub const PROBLEM_DETAILS_CRATE_VERSION: &'static str = env!("CARGO_PKG_VERSION");
        }
    })
}

// ── Arm generation ────────────────────────────────────────────────────────────

fn method_arms(
    metas: &[VariantMeta],
    f: &dyn Fn(&VariantMeta) -> TokenStream2,
) -> Vec<TokenStream2> {
    metas
        .iter()
        .map(|m| {
            let p = pat(&m.ident, &m.fields);
            let val = f(m);
            quote! { #p => #val, }
        })
        .collect()
}

/// Emits `match self { Self::Foo(inner) => inner.method() }` for a delegate newtype variant.
fn delegate_call(m: &VariantMeta, method: TokenStream2) -> TokenStream2 {
    let ident = &m.ident;
    quote! { { match self { Self::#ident(inner) => inner.#method, _ => unreachable!() } } }
}

// ── Parsing ───────────────────────────────────────────────────────────────────

fn parse_variant(v: &Variant) -> syn::Result<VariantMeta> {
    let attr = v
        .attrs
        .iter()
        .find(|a| a.path().is_ident("problem"))
        .ok_or_else(|| {
            Error::new_spanned(
                v,
                "missing #[problem(...)] — use #[problem(delegate)] for delegating variants",
            )
        })?;

    // Check for bare #[problem(delegate)]
    let args = attr.parse_args_with(Punctuated::<Meta, Comma>::parse_terminated)?;
    let is_delegate = args.len() == 1 && args[0].path().is_ident("delegate");

    if is_delegate {
        // Must be a newtype variant
        match &v.fields {
            Fields::Unnamed(f) if f.unnamed.len() == 1 => {}
            _ => {
                return Err(Error::new_spanned(
                    v,
                    "#[problem(delegate)] requires a newtype variant: Foo(InnerType)",
                ));
            }
        }
        return Ok(VariantMeta {
            ident: v.ident.clone(),
            fields: v.fields.clone(),
            kind: VariantKind::Delegate,
        });
    }

    // Leaf variant — parse slug, title, http_status
    let doc = v
        .attrs
        .iter()
        .filter_map(|a| {
            if !a.path().is_ident("doc") {
                return None;
            }
            if let Meta::NameValue(nv) = &a.meta
                && let Expr::Lit(ExprLit {
                    lit: Lit::Str(s), ..
                }) = &nv.value
            {
                return Some(s.value().trim().to_owned());
            }
            None
        })
        .collect::<Vec<_>>()
        .join(" ");

    let mut slug = None;
    let mut title = None;
    let mut http_status = None;

    for meta in &args {
        let Meta::NameValue(nv) = meta else {
            return Err(Error::new_spanned(meta, "expected key = value"));
        };
        let val = &nv.value;
        if nv.path.is_ident("slug") {
            slug = Some(str_lit(val, "slug")?);
        } else if nv.path.is_ident("title") {
            title = Some(str_lit(val, "title")?);
        } else if nv.path.is_ident("http_status") {
            http_status = Some(int_lit(val, "http_status")?);
        } else {
            return Err(Error::new_spanned(
                &nv.path,
                "unknown key; expected slug, title, http_status, or delegate",
            ));
        }
    }

    Ok(VariantMeta {
        ident: v.ident.clone(),
        fields: v.fields.clone(),
        kind: VariantKind::Leaf {
            slug: slug.ok_or_else(|| Error::new_spanned(attr, "missing slug"))?,
            title: title.ok_or_else(|| Error::new_spanned(attr, "missing title"))?,
            http_status: http_status
                .ok_or_else(|| Error::new_spanned(attr, "missing http_status"))?,
            doc,
        },
    })
}

fn str_lit(expr: &Expr, key: &str) -> syn::Result<String> {
    if let Expr::Lit(ExprLit {
        lit: Lit::Str(s), ..
    }) = expr
    {
        Ok(s.value())
    } else {
        Err(Error::new_spanned(
            expr,
            format!("`{key}` must be a string literal"),
        ))
    }
}

fn int_lit(expr: &Expr, key: &str) -> syn::Result<u16> {
    if let Expr::Lit(ExprLit {
        lit: Lit::Int(n), ..
    }) = expr
    {
        n.base10_parse()
            .map_err(|_| Error::new_spanned(expr, format!("`{key}` must be a u16")))
    } else {
        Err(Error::new_spanned(
            expr,
            format!("`{key}` must be an integer literal"),
        ))
    }
}

fn pat(ident: &syn::Ident, fields: &Fields) -> TokenStream2 {
    match fields {
        Fields::Unit => quote! { Self::#ident },
        Fields::Unnamed(_) => quote! { Self::#ident(..) },
        Fields::Named(_) => quote! { Self::#ident { .. } },
    }
}

// ── Manifest ──────────────────────────────────────────────────────────────────

fn manifest_json(metas: &[&VariantMeta]) -> String {
    let entries: Vec<String> = metas
        .iter()
        .filter_map(|m| {
            if let VariantKind::Leaf {
                slug,
                title,
                http_status,
                doc,
            } = &m.kind
            {
                Some(format!(
                    r#"{{"slug":{},"title":{},"http_status":{},"doc":{}}}"#,
                    json_str(slug),
                    json_str(title),
                    http_status,
                    json_str(doc)
                ))
            } else {
                None
            }
        })
        .collect();
    format!("[{}]", entries.join(","))
}

fn json_str(s: &str) -> String {
    format!(
        "\"{}\"",
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    )
}
