//! CDDL (RFC 8610) generation support for minicbor wire types.
//!
//! Each wire type derives [`ToCddl`], which yields:
//! - [`ToCddl::cddl_ref`] — the name used when this type appears as a field.
//! - [`ToCddl::cddl_definition`] — the full `name = ...` rule, or `None` for primitives.
//!
//! Assemble a complete spec by deriving [`CddlSpec`] on a marker struct:
//!
//! ```ignore
//! #[derive(CddlSpec)]
//! #[cddl_spec(
//!     naming = "two",
//!     types(
//!         quote::Error,
//!         sync::Response,
//!         squash::Proposal as "squash-proposal",
//!     )
//! )]
//! pub struct MySpec;
//!
//! fn main() {
//!     print!("{}", MySpec::cddl());
//! }
//! ```
//!
//! See [`cuddly_derive`] for the full attribute reference.

#[cfg(feature = "derive")]
pub use cuddly_derive::{CddlSpec, ToCddl};

// ---------------------------------------------------------------------------
// Core traits
// ---------------------------------------------------------------------------

/// Implemented by every wire type via `#[derive(ToCddl)]`.
pub trait ToCddl {
    /// The CDDL type reference used when this type appears as a field.
    /// Primitives return the CDDL prelude name (`uint`, `text`, …).
    /// Named types return their kebab-case rule name.
    fn cddl_ref() -> String;

    /// The full CDDL rule definition, e.g. `"my-type = [uint, text]"`.
    /// Returns `None` for primitives and generics that have no standalone rule.
    fn cddl_definition() -> Option<String>;
}

/// Implemented by the marker struct produced by `#[derive(CddlSpec)]`.
pub trait CddlSpec {
    /// Collect, rename, deduplicate, and concatenate all CDDL definitions.
    fn cddl() -> String;
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

// Vec<u8> → "bytes". Rust has no specialisation on stable, so Vec<T> cannot
// have a blanket impl alongside this. For other sequence types use
// #[cddl(ty = "[* my-element]")] at the field level.
impl ToCddl for Vec<u8> {
    fn cddl_ref() -> String {
        "bytes".to_string()
    }
    fn cddl_definition() -> Option<String> {
        None
    }
}

// ---------------------------------------------------------------------------
// Generic impls
// ---------------------------------------------------------------------------

impl<T: ToCddl> ToCddl for Option<T> {
    fn cddl_ref() -> String {
        format!("{} / null", T::cddl_ref())
    }
    fn cddl_definition() -> Option<String> {
        None
    }
}

// ---------------------------------------------------------------------------
// Rule renaming
// ---------------------------------------------------------------------------

/// Replace all occurrences of a bare CDDL rule name with a new name in a
/// definition string.
///
/// Called by the `CddlSpec` derive to apply the chosen naming strategy (or
/// an explicit `as "name"` override) without requiring any annotation on the
/// type itself.
///
/// Matches `from` only as a whole word — bounded by start-of-line, whitespace,
/// `,`, `[`, `]`, `/`, or end-of-string — so `error` does not match inside
/// `quote-error`.
pub fn rename_rule(def: &str, from: &str, to: &str) -> String {
    if from == to {
        return def.to_string();
    }
    // Simple character-by-character scan; avoids pulling in `regex`.
    let bytes = def.as_bytes();
    let from_b = from.as_bytes();
    let flen = from_b.len();
    let mut out = String::with_capacity(def.len() + 16);
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i..].starts_with(from_b) {
            let left_ok = i == 0 || is_boundary(bytes[i - 1]);
            let right_ok = i + flen >= bytes.len() || is_boundary(bytes[i + flen]);
            if left_ok && right_ok {
                out.push_str(to);
                i += flen;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

#[inline]
fn is_boundary(b: u8) -> bool {
    matches!(
        b,
        b' ' | b'\n' | b'\t' | b',' | b'[' | b']' | b'/' | b'=' | b';' | b'('
    )
}

#[cfg(all(test, feature = "derive"))]
mod tests {
    use super::*;

    #[derive(ToCddl)]
    struct Foo<T> {
        _index: u64,
        _latch: T,
    }

    #[test]
    fn generic_struct_derives_correctly() {
        let out = Foo::<u64>::cddl_ref();
        assert!(!out.is_empty());
    }
}
