use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use syn::{Attribute, Expr, Fields, Item, Lit, Meta, Type, parse_file};

// ---------------------------------------------------------------------------
// Domain types
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
struct EndpointModule {
    /// Module path segments, e.g. ["auth", "pay"]
    mod_path: Vec<String>,
    /// The ENDPOINT const defined in this module (just the segment, e.g. "/pay")
    endpoint_const: Option<String>,
    /// Resolved full path (built by chaining parent paths).
    /// Present for both endpoints and intermediate path modules.
    resolved_path: Option<String>,
    /// Docs on the module (//! lines)
    module_docs: Vec<String>,
    /// If a `Body` struct exists → POST, otherwise GET
    body: Option<StructInfo>,
    /// Response struct
    response: Option<StructInfo>,
    /// Error enums (ProblemDetail)
    errors: Vec<EnumInfo>,
    /// Error type alias, e.g. `type Error = auth::Error<DomainError>`
    error_type: Option<String>,
    /// Domain error type alias, e.g. `type DomainError = common::commit::Error`
    domain_error_type: Option<String>,
    /// Other public structs (helpers, nested types)
    other_structs: Vec<StructInfo>,
    /// Child endpoint modules declared via `pub mod foo;`
    child_modules: Vec<String>,
}

impl EndpointModule {
    /// True if this module defines an endpoint.
    /// An endpoint must have a resolved PATH and at least one of:
    /// - a Response type (GET endpoints, and POST endpoints with their own response)
    /// - a Body type (POST endpoints — response may live in a shared module)
    fn is_endpoint(&self) -> bool {
        self.resolved_path.is_some() && (self.response.is_some() || self.body.is_some())
    }

    /// True if this module is an intermediate path (has PATH but no Response or Body)
    fn is_intermediate(&self) -> bool {
        self.resolved_path.is_some() && self.response.is_none() && self.body.is_none()
    }

    /// True if this module has any documentable content
    fn has_content(&self) -> bool {
        self.is_endpoint()
            || self.body.is_some()
            || self.response.is_some()
            || !self.errors.is_empty()
            || !self.other_structs.is_empty()
            || !self.child_modules.is_empty()
    }
}

#[derive(Debug)]
struct StructInfo {
    name: String,
    docs: Vec<String>,
    generics: Vec<String>,
    fields: Vec<FieldInfo>,
}

#[derive(Debug)]
struct FieldInfo {
    name: String,
    ty: String,
    docs: Vec<String>,
    cbor_index: Option<String>,
    serde_annotation: Option<String>,
}

#[derive(Debug)]
struct EnumInfo {
    name: String,
    docs: Vec<String>,
    variants: Vec<VariantInfo>,
}

#[derive(Debug)]
struct VariantInfo {
    name: String,
    docs: Vec<String>,
    slug: Option<String>,
    title: Option<String>,
    http_status: Option<String>,
    is_delegate: bool,
    inner_type: Option<String>,
}

// ---------------------------------------------------------------------------
// Attribute helpers
// ---------------------------------------------------------------------------

fn extract_doc_lines(attrs: &[Attribute]) -> Vec<String> {
    attrs
        .iter()
        .filter_map(|attr| {
            if !attr.path().is_ident("doc") {
                return None;
            }
            if let Meta::NameValue(nv) = &attr.meta {
                if let Expr::Lit(expr_lit) = &nv.value {
                    if let Lit::Str(s) = &expr_lit.lit {
                        return Some(s.value().trim().to_string());
                    }
                }
            }
            None
        })
        .collect()
}

/// Extract #[n(X)] or #[cbor(n(X), ...)] index
fn extract_cbor_index(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        // #[n(0)]
        if attr.path().is_ident("n") {
            if let Ok(expr) = attr.parse_args::<Expr>() {
                return Some(expr_to_string(&expr));
            }
        }
        // #[cbor(n(0), ...)]
        if attr.path().is_ident("cbor") {
            // Don't use `?` here — it would short-circuit the whole function.
            if let Ok(list) = attr.meta.require_list() {
                let tokens = list.tokens.to_string();
                if let Some(start) = tokens.find("n(") {
                    let rest = &tokens[start + 2..];
                    if let Some(end) = rest.find(')') {
                        return Some(rest[..end].trim().to_string());
                    }
                }
            }
        }
    }
    None
}

/// Extract serde_with / serde_as annotations
fn extract_serde_annotation(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident("serde_as") {
            if let Ok(list) = attr.meta.require_list() {
                let tokens = list.tokens.to_string();
                if let Some(pos) = tokens.find("as = ") {
                    let rest = &tokens[pos + 5..];
                    let cleaned = rest.trim_matches('"').trim().to_string();
                    return Some(cleaned);
                }
            }
        }
    }
    None
}

/// Extract #[problem(...)] attributes from enum variants
fn extract_problem_attrs(
    attrs: &[Attribute],
) -> (Option<String>, Option<String>, Option<String>, bool) {
    let mut slug = None;
    let mut title = None;
    let mut http_status = None;
    let mut is_delegate = false;

    for attr in attrs {
        if !attr.path().is_ident("problem") {
            continue;
        }
        let tokens = match attr.meta.require_list() {
            Ok(list) => list.tokens.to_string(),
            Err(_) => continue,
        };

        if tokens.contains("delegate") {
            is_delegate = true;
            continue;
        }

        for part in tokens.split(',') {
            let part = part.trim();
            if let Some(val) = extract_kv(part, "slug") {
                slug = Some(val);
            } else if let Some(val) = extract_kv(part, "title") {
                title = Some(val);
            } else if let Some(val) = extract_kv(part, "http_status") {
                http_status = Some(val);
            }
        }
    }

    (slug, title, http_status, is_delegate)
}

fn extract_kv(s: &str, key: &str) -> Option<String> {
    let prefix = format!("{} = ", key);
    let prefix_no_space = format!("{}=", key);
    let rest = if s.starts_with(&prefix) {
        &s[prefix.len()..]
    } else if s.starts_with(&prefix_no_space) {
        &s[prefix_no_space.len()..]
    } else {
        return None;
    };
    Some(rest.trim_matches('"').trim().to_string())
}

fn expr_to_string(expr: &Expr) -> String {
    quote::quote!(#expr).to_string()
}

fn type_to_string(ty: &Type) -> String {
    quote::quote!(#ty)
        .to_string()
        .replace(" < ", "<")
        .replace(" > ", ">")
        .replace("< ", "<")
        .replace(" >", ">")
        .replace(" ,", ",")
}

fn extract_generics(generics: &syn::Generics) -> Vec<String> {
    generics
        .params
        .iter()
        .filter_map(|p| {
            if let syn::GenericParam::Type(tp) = p {
                Some(tp.ident.to_string())
            } else {
                None
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

fn parse_struct(item: &syn::ItemStruct) -> StructInfo {
    let fields = match &item.fields {
        Fields::Named(named) => named
            .named
            .iter()
            .map(|f| FieldInfo {
                name: f.ident.as_ref().map_or("_".into(), |i| i.to_string()),
                ty: type_to_string(&f.ty),
                docs: extract_doc_lines(&f.attrs),
                cbor_index: extract_cbor_index(&f.attrs),
                serde_annotation: extract_serde_annotation(&f.attrs),
            })
            .collect(),
        _ => vec![],
    };

    StructInfo {
        name: item.ident.to_string(),
        docs: extract_doc_lines(&item.attrs),
        generics: extract_generics(&item.generics),
        fields,
    }
}

fn parse_enum(item: &syn::ItemEnum) -> EnumInfo {
    let variants = item
        .variants
        .iter()
        .map(|v| {
            let (slug, title, http_status, is_delegate) = extract_problem_attrs(&v.attrs);
            let inner_type = match &v.fields {
                Fields::Unnamed(u) if u.unnamed.len() == 1 => {
                    Some(type_to_string(&u.unnamed[0].ty))
                }
                _ => None,
            };
            VariantInfo {
                name: v.ident.to_string(),
                docs: extract_doc_lines(&v.attrs),
                slug,
                title,
                http_status,
                is_delegate,
                inner_type,
            }
        })
        .collect();

    EnumInfo {
        name: item.ident.to_string(),
        docs: extract_doc_lines(&item.attrs),
        variants,
    }
}

fn has_derive(attrs: &[Attribute], name: &str) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("derive")
            && attr
                .meta
                .require_list()
                .ok()
                .map(|l| l.tokens.to_string().contains(name))
                .unwrap_or(false)
    })
}

/// Extract the string value of a `const ENDPOINT: &str = "/foo";` or `const PATH: &str = "...";`
fn extract_string_const(items: &[Item], name: &str) -> Option<String> {
    for item in items {
        if let Item::Const(c) = item {
            if c.ident == name {
                if let Expr::Lit(expr_lit) = c.expr.as_ref() {
                    if let Lit::Str(s) = &expr_lit.lit {
                        return Some(s.value());
                    }
                }
                // Macro call (concatcp!) — we know it exists but can't evaluate
                return Some(String::new());
            }
        }
    }
    None
}

fn analyze_file(path: &Path, mod_path: &[String]) -> EndpointModule {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => return EndpointModule::default(),
    };

    let file = match parse_file(&source) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Parse error in {}: {}", path.display(), e);
            return EndpointModule::default();
        }
    };

    let mut ep = EndpointModule {
        mod_path: mod_path.to_vec(),
        ..Default::default()
    };

    // Module-level docs
    ep.module_docs = extract_doc_lines(&file.attrs);

    // Extract ENDPOINT const (the segment, e.g. "/pay")
    ep.endpoint_const = extract_string_const(&file.items, "ENDPOINT");

    // If there's a PATH const, this module participates in routing.
    // It may be an endpoint (if it also has Response) or just an intermediate path.
    // resolved_path will be built later by resolve_paths.
    let has_path = file
        .items
        .iter()
        .any(|item| matches!(item, Item::Const(c) if c.ident == "PATH"));
    if has_path {
        // Placeholder — resolve_paths will fill this in
        ep.resolved_path = Some(String::new());
    }

    for item in &file.items {
        match item {
            Item::Struct(s)
                if has_derive(&s.attrs, "Serialize")
                    || has_derive(&s.attrs, "Encode")
                    || s.ident == "Body"
                    || s.ident == "Response" =>
            {
                let info = parse_struct(s);
                match s.ident.to_string().as_str() {
                    "Body" => ep.body = Some(info),
                    "Response" => ep.response = Some(info),
                    _ => ep.other_structs.push(info),
                }
            }
            // Type aliases: `pub type Response = ChequeProposal;`
            Item::Type(t) => {
                let name = t.ident.to_string();
                let target = type_to_string(&t.ty);
                let mut docs = extract_doc_lines(&t.attrs);
                docs.push(format!("Alias for `{}`.", target));
                let info = StructInfo {
                    name: name.clone(),
                    docs,
                    generics: extract_generics(&t.generics),
                    fields: vec![],
                };
                match name.as_str() {
                    "Body" => ep.body = Some(info),
                    "Response" => ep.response = Some(info),
                    "Error" => ep.error_type = Some(target),
                    "DomainError" => ep.domain_error_type = Some(target),
                    _ => {}
                }
            }
            Item::Enum(e) if has_derive(&e.attrs, "ProblemDetail") => {
                ep.errors.push(parse_enum(e));
            }
            Item::Mod(m) if m.content.is_none() => {
                ep.child_modules.push(m.ident.to_string());
            }
            _ => {}
        }
    }

    // Sort for deterministic, alphabetical output
    ep.child_modules.sort();
    ep.other_structs.sort_by(|a, b| a.name.cmp(&b.name));
    ep.errors.sort_by(|a, b| a.name.cmp(&b.name));

    ep
}

/// Walk the module tree rooted at `src_dir`.
/// Only follows directories that correspond to declared `pub mod` children,
/// starting from lib.rs (or mod.rs).
fn discover_modules(src_dir: &Path) -> BTreeMap<Vec<String>, PathBuf> {
    let mut result = BTreeMap::new();

    // Find the root file
    let root = if src_dir.join("lib.rs").exists() {
        src_dir.join("lib.rs")
    } else if src_dir.join("mod.rs").exists() {
        src_dir.join("mod.rs")
    } else if src_dir.join("main.rs").exists() {
        // Skip main.rs for doc gen — it's the generator itself
        if src_dir.join("lib.rs").exists() {
            src_dir.join("lib.rs")
        } else {
            eprintln!("No lib.rs or mod.rs found in {}", src_dir.display());
            return result;
        }
    } else {
        eprintln!("No lib.rs or mod.rs found in {}", src_dir.display());
        return result;
    };

    // Parse root to find declared child modules
    let root_ep = analyze_file_for_children(&root);
    result.insert(vec![], root);

    for child in &root_ep {
        walk_declared_modules(src_dir, &[child.clone()], child, &mut result);
    }

    result
}

/// Parse a file just to extract `pub mod foo;` declarations
fn analyze_file_for_children(path: &Path) -> Vec<String> {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    let file = match parse_file(&source) {
        Ok(f) => f,
        Err(_) => return vec![],
    };

    file.items
        .iter()
        .filter_map(|item| {
            if let Item::Mod(m) = item {
                if m.content.is_none() {
                    return Some(m.ident.to_string());
                }
            }
            None
        })
        .collect()
}

fn walk_declared_modules(
    src_dir: &Path,
    mod_path: &[String],
    mod_name: &str,
    out: &mut BTreeMap<Vec<String>, PathBuf>,
) {
    // A module `foo` can be at `foo.rs` or `foo/mod.rs`
    let file_path = src_dir.join(format!("{}.rs", mod_name));
    let dir_path = src_dir.join(mod_name);
    let dir_mod_path = dir_path.join("mod.rs");

    let actual_path;
    let child_src_dir;

    if dir_mod_path.exists() {
        actual_path = dir_mod_path;
        child_src_dir = dir_path.clone();
    } else if file_path.exists() {
        actual_path = file_path;
        child_src_dir = src_dir.to_path_buf();
    } else {
        eprintln!(
            "Module {} not found (tried {} and {})",
            mod_path.join("::"),
            file_path.display(),
            dir_mod_path.display()
        );
        return;
    }

    out.insert(mod_path.to_vec(), actual_path.clone());

    // Recurse into declared children
    let children = analyze_file_for_children(&actual_path);
    let next_src_dir = if dir_path.is_dir() {
        &dir_path
    } else {
        &child_src_dir
    };

    for child in &children {
        let mut child_mod_path = mod_path.to_vec();
        child_mod_path.push(child.clone());
        walk_declared_modules(next_src_dir, &child_mod_path, child, out);
    }
}

/// Resolve full HTTP paths by chaining ENDPOINT consts.
/// lib.rs defines PATH = "", each child defines ENDPOINT = "/foo"
/// and PATH = concatcp!(super::PATH, ENDPOINT).
/// We reconstruct this chain.
fn resolve_paths(modules: &mut BTreeMap<Vec<String>, EndpointModule>) {
    // Collect endpoints first (immutable borrow)
    let endpoints: BTreeMap<Vec<String>, Option<String>> = modules
        .iter()
        .map(|(k, v)| (k.clone(), v.endpoint_const.clone()))
        .collect();

    // Now resolve each module's path
    for (mod_path, ep) in modules.iter_mut() {
        if ep.resolved_path.is_none() {
            continue;
        }

        // Build path by concatenating ancestors' ENDPOINT consts
        let mut segments = Vec::new();
        // The root (lib.rs, path=[]) has ENDPOINT=""
        for i in 1..=mod_path.len() {
            let ancestor = &mod_path[..i];
            if let Some(Some(endpoint)) = endpoints.get(ancestor) {
                if !endpoint.is_empty() {
                    segments.push(endpoint.clone());
                }
            }
        }

        let full_path = segments.join("");
        ep.resolved_path = Some(if full_path.is_empty() {
            "/".to_string()
        } else {
            full_path
        });
    }
}

// ---------------------------------------------------------------------------
// Markdown generation
// ---------------------------------------------------------------------------

/// Format module path for display: ["auth", "pay"] → "auth::pay"
fn mod_display(mod_path: &[String]) -> String {
    if mod_path.is_empty() {
        "root".to_string()
    } else {
        mod_path.join("::")
    }
}

/// Check if a module is under the auth subtree
fn is_authed(mod_path: &[String]) -> bool {
    mod_path.first().map(|s| s.as_str()) == Some("auth")
}

/// Strip leading `#` markers from doc lines so they don't create
/// spurious markdown headers inside a section.
fn sanitize_doc_line(line: &str) -> String {
    if line.starts_with('#') {
        let trimmed = line.trim_start_matches('#').trim_start();
        format!("**{}**", trimmed)
    } else {
        line.to_string()
    }
}

/// Render doc lines to markdown, preserving empty lines as paragraph breaks.
/// Strips leading/trailing blank lines but keeps internal ones.
fn render_docs(out: &mut String, docs: &[String], sanitize_headers: bool) {
    // Trim leading and trailing empty lines
    let start = docs.iter().position(|l| !l.is_empty());
    let end = docs.iter().rposition(|l| !l.is_empty());
    let (start, end) = match (start, end) {
        (Some(s), Some(e)) => (s, e),
        _ => return, // all empty or no docs
    };

    for line in &docs[start..=end] {
        if line.is_empty() {
            out.push('\n');
        } else if sanitize_headers {
            out.push_str(&sanitize_doc_line(line));
            out.push('\n');
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    out.push('\n');
}

fn render_markdown(modules: &BTreeMap<Vec<String>, EndpointModule>) -> String {
    let mut out = String::new();

    out.push_str("# API Reference\n\n");
    out.push_str("*Auto-generated from source using `syn`.*\n\n");
    out.push_str("All wire types support JSON (`serde`) and CBOR (`minicbor`) serialization.\n");
    out.push_str("Errors use the Problem Details convention.\n\n");
    out.push_str("---\n\n");

    // ── Endpoint summary table ──────────────────────────────────────────
    out.push_str("## Endpoints\n\n");
    out.push_str("| Path | Module | Method | Auth |\n");
    out.push_str("|------|--------|--------|------|\n");

    for (mod_path, ep) in modules {
        if !ep.is_endpoint() {
            continue;
        }
        // Skip root (lib.rs) — it's not a real endpoint
        if mod_path.is_empty() {
            continue;
        }

        let display = mod_display(mod_path);
        let path_val = ep.resolved_path.as_deref().unwrap_or("—");
        let method = if ep.body.is_some() { "`POST`" } else { "`GET`" };
        let auth = if is_authed(mod_path) { "Yes" } else { "No" };

        out.push_str(&format!(
            "| `{}` | `{}` | {} | {} |\n",
            path_val, display, method, auth
        ));
    }
    out.push('\n');

    // ── Detail sections ─────────────────────────────────────────────────
    for (mod_path, ep) in modules {
        // Skip root
        if mod_path.is_empty() {
            continue;
        }
        // Skip utility-only modules with no endpoint content
        // (but still render if they have errors/structs to document)
        if !ep.has_content() {
            continue;
        }

        let display = mod_display(mod_path);
        out.push_str(&format!("---\n\n## `{}`\n\n", display));

        // Module docs (sanitize headers)
        render_docs(&mut out, &ep.module_docs, true);

        // Method + path (only for actual endpoints, not intermediate paths)
        if ep.is_endpoint() {
            if let Some(path) = &ep.resolved_path {
                let method = if ep.body.is_some() { "POST" } else { "GET" };
                out.push_str(&format!("**`{} {}`**\n\n", method, path));
            }
        }

        // Body
        if let Some(body) = &ep.body {
            render_struct(&mut out, body, "Request Body");
        }

        // Response
        if let Some(resp) = &ep.response {
            render_struct(&mut out, resp, "Response");
        }

        // Other structs
        for s in &ep.other_structs {
            render_struct(&mut out, s, &s.name);
        }

        // Errors (inline enums)
        for e in &ep.errors {
            render_enum(&mut out, e);
        }

        // Error type aliases
        if let Some(err) = &ep.error_type {
            out.push_str(&format!("**Error:** `{}`", err));
            if let Some(domain) = &ep.domain_error_type {
                out.push_str(&format!(" where domain = `{}`", domain));
            }
            out.push_str("\n\n");
        }

        // Child modules
        if !ep.child_modules.is_empty() {
            out.push_str("### Submodules\n\n");
            for child in &ep.child_modules {
                let mut child_path = mod_path.clone();
                child_path.push(child.clone());
                let child_display = mod_display(&child_path);
                out.push_str(&format!(
                    "- [`{}`](#{})\n",
                    child_display,
                    slug_anchor(&child_display)
                ));
            }
            out.push('\n');
        }
    }

    // ── Shared error types ──────────────────────────────────────────────
    let shared_errors: Vec<_> = modules
        .iter()
        .filter(|(path, ep)| {
            // Modules that only define errors and aren't endpoints themselves
            !ep.is_endpoint() && !ep.errors.is_empty()
        })
        .collect();

    if !shared_errors.is_empty() {
        out.push_str("---\n\n## Shared Error Types\n\n");
        for (mod_path, ep) in shared_errors {
            let display = mod_display(mod_path);
            render_docs(&mut out, &ep.module_docs, true);
            for e in &ep.errors {
                let qualified = format!("{}::{}", display, e.name);
                out.push_str(&format!("### `{}`\n\n", qualified));
                render_docs(&mut out, &e.docs, false);
                out.push_str("| Variant | Slug | Status | Description |\n");
                out.push_str("|---------|------|--------|-------------|\n");
                for v in &e.variants {
                    render_variant_row(&mut out, v);
                }
                out.push('\n');
            }
        }
    }

    out
}

/// Generate a markdown anchor slug from a section title
fn slug_anchor(s: &str) -> String {
    s.to_lowercase().replace("::", "").replace(' ', "-")
}

fn render_struct(out: &mut String, s: &StructInfo, label: &str) {
    let generics = if s.generics.is_empty() {
        String::new()
    } else {
        format!("<{}>", s.generics.join(", "))
    };

    out.push_str(&format!("### {}{}\n\n", label, generics));

    render_docs(out, &s.docs, false);

    if s.fields.is_empty() {
        if s.docs.is_empty() {
            out.push_str("*(unit type — no fields)*\n\n");
        }
        return;
    }

    let has_cbor = s.fields.iter().any(|f| f.cbor_index.is_some());
    let has_serde = s.fields.iter().any(|f| f.serde_annotation.is_some());

    // Header
    out.push_str("| Field | Type |");
    if has_cbor {
        out.push_str(" CBOR |");
    }
    if has_serde {
        out.push_str(" JSON encoding |");
    }
    out.push_str(" Description |\n");

    out.push_str("|-------|------|");
    if has_cbor {
        out.push_str("------|");
    }
    if has_serde {
        out.push_str("----------------|");
    }
    out.push_str("-------------|\n");

    for f in &s.fields {
        let doc = f.docs.join(" ");
        out.push_str(&format!("| `{}` | `{}` |", f.name, f.ty));
        if has_cbor {
            out.push_str(&format!(" {} |", f.cbor_index.as_deref().unwrap_or("—")));
        }
        if has_serde {
            out.push_str(&format!(
                " {} |",
                f.serde_annotation.as_deref().unwrap_or("—")
            ));
        }
        out.push_str(&format!(" {} |\n", doc));
    }
    out.push('\n');
}

fn render_enum(out: &mut String, e: &EnumInfo) {
    out.push_str(&format!("### `{}`\n\n", e.name));

    render_docs(out, &e.docs, false);

    out.push_str("| Variant | Slug | Status | Description |\n");
    out.push_str("|---------|------|--------|-------------|\n");

    for v in &e.variants {
        render_variant_row(out, v);
    }
    out.push('\n');
}

fn render_variant_row(out: &mut String, v: &VariantInfo) {
    let doc = v.docs.join(" ");
    let slug = if v.is_delegate {
        format!(
            "*delegates to `{}`*",
            v.inner_type.as_deref().unwrap_or("?")
        )
    } else {
        v.slug
            .as_ref()
            .map(|s| format!("`{}`", s))
            .unwrap_or_else(|| "—".to_string())
    };
    let status = v.http_status.as_deref().unwrap_or("—");
    out.push_str(&format!(
        "| `{}` | {} | {} | {} |\n",
        v.name, slug, status, doc
    ));
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let src_dir = std::env::args().nth(1).unwrap_or_else(|| "src".to_string());
    let output_path = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "API.md".to_string());

    let src = Path::new(&src_dir);
    if !src.exists() {
        eprintln!("Source directory '{}' not found", src_dir);
        std::process::exit(1);
    }

    let file_map = discover_modules(src);
    let mut modules = BTreeMap::new();

    for (mod_path, file_path) in &file_map {
        let ep = analyze_file(file_path, mod_path);
        modules.insert(mod_path.clone(), ep);
    }

    // Resolve full HTTP paths by chaining ENDPOINT consts
    resolve_paths(&mut modules);

    let endpoint_count = modules.values().filter(|e| e.is_endpoint()).count();
    let markdown = render_markdown(&modules);

    fs::write(&output_path, &markdown).expect("Failed to write output");
    println!("Wrote {} ({} bytes)", output_path, markdown.len());
    println!(
        "Discovered {} modules, {} endpoints",
        modules.len(),
        endpoint_count
    );
}
