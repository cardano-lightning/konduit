//! CDDL (RFC 8610) generation support for konduit wire types.
//!
//! Each wire type derives [`ToCddl`], which yields:
//! - `cddl_ref()` — the name used when the type appears as a field in another definition.
//! - `cddl_definition()` — the full `name = ...` CDDL rule, or `None` for primitives.
//!
//! A generator binary collects `cddl_definition()` from all types and assembles the spec.

#[cfg(feature = "derive")]
pub use konduit_cddl_derive::ToCddl;

pub trait ToCddl {
    /// The CDDL type reference — used when this type appears as a field.
    /// For primitives this is the CDDL prelude name (`uint`, `text`, …).
    /// For named types this is the kebab-case rule name.
    fn cddl_ref() -> String;

    /// The full CDDL rule definition, e.g. `"my-type = [uint, text]"`.
    /// Returns `None` for primitives and generics that have no standalone rule.
    fn cddl_definition() -> Option<String>;
}

// ---------------------------------------------------------------------------
// Primitive impls
// ---------------------------------------------------------------------------

macro_rules! impl_primitive {
    ($cddl:literal => $($t:ty),+) => {
        $(impl ToCddl for $t {
            fn cddl_ref() -> String { $cddl.to_string() }
            fn cddl_definition() -> Option<String> { None }
        })+
    };
}

impl_primitive!("uint" => u8, u16, u32, u64, usize);
impl_primitive!("int"  => i8, i16, i32, i64, isize);
impl_primitive!("bool" => bool);
impl_primitive!("text" => String);
impl_primitive!("bytes" => [u8]);

// ---------------------------------------------------------------------------
// Generic impls
// ---------------------------------------------------------------------------

impl<T: ToCddl> ToCddl for Vec<T> {
    fn cddl_ref() -> String {
        format!("[* {}]", T::cddl_ref())
    }
    fn cddl_definition() -> Option<String> {
        None
    }
}

impl<T: ToCddl> ToCddl for Option<T> {
    fn cddl_ref() -> String {
        format!("{} / null", T::cddl_ref())
    }
    fn cddl_definition() -> Option<String> {
        None
    }
}

// ---------------------------------------------------------------------------
// Collector
// ---------------------------------------------------------------------------

/// Collect CDDL definitions from a list of `cddl_definition()` calls, filtering
/// out `None` (primitives) and deduplicating.
///
/// Usage in a generator binary:
/// ```ignore
/// let spec = konduit_cddl::collect(&[
///     BackingView::cddl_definition,
///     DepthBucket::cddl_definition,
///     sync::Response::cddl_definition,
///     sync::Error::cddl_definition,
///     // ...
/// ]);
/// println!("{}", spec);
/// ```
pub fn collect(defs: &[fn() -> Option<String>]) -> String {
    let mut seen = std::collections::HashSet::new();
    let mut out = String::new();
    for def in defs {
        if let Some(d) = def() {
            // Deduplicate by the rule name (everything before " =")
            let key = d.split(" =").next().unwrap_or(&d).trim().to_string();
            if seen.insert(key) {
                out.push_str(&d);
                out.push_str("\n\n");
            }
        }
    }
    out
}
