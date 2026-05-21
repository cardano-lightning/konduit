//! Validates that `cuddly`'s primitive and generic `ToCddl` impls produce
//! CDDL that actually matches what minicbor puts on the wire.
//!
//! Each test:
//!   1. Encodes an arbitrary value with minicbor.
//!   2. Validates the bytes against the CDDL emitted by `ToCddl::cddl_ref()`.
//!
//! Validator: `cddl-cat`, which handles both definite and indefinite-length
//! CBOR arrays.

use cddl_cat::validate_cbor_bytes;
use cuddly::ToCddl;
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn encode<T: minicbor::Encode<()>>(val: &T) -> Vec<u8> {
    minicbor::to_vec(val).expect("minicbor encoding failed")
}

/// Validate `cbor` against a minimal CDDL document wrapping `T::cddl_ref()`.
/// `cddl-cat` requires a named rule as the entry point.
fn validate<T: ToCddl>(cbor: &[u8]) {
    let cddl_ref = T::cddl_ref();
    let spec = format!("root = {}\n", cddl_ref);
    validate_cbor_bytes("root", &spec, cbor)
        .unwrap_or_else(|e| panic!("CDDL validation failed for `{}`: {:?}", cddl_ref, e));
}

// ---------------------------------------------------------------------------
// Unsigned integers → uint
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn u8_is_uint(v in any::<u8>()) {
        validate::<u8>(&encode(&v));
    }

    #[test]
    fn u16_is_uint(v in any::<u16>()) {
        validate::<u16>(&encode(&v));
    }

    #[test]
    fn u32_is_uint(v in any::<u32>()) {
        validate::<u32>(&encode(&v));
    }

    #[test]
    fn u64_is_uint(v in any::<u64>()) {
        validate::<u64>(&encode(&v));
    }

    #[test]
    fn usize_is_uint(v in any::<usize>()) {
        validate::<usize>(&encode(&v));
    }
}

// ---------------------------------------------------------------------------
// Signed integers → int
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn i8_is_int(v in any::<i8>()) {
        validate::<i8>(&encode(&v));
    }

    #[test]
    fn i16_is_int(v in any::<i16>()) {
        validate::<i16>(&encode(&v));
    }

    #[test]
    fn i32_is_int(v in any::<i32>()) {
        validate::<i32>(&encode(&v));
    }

    #[test]
    fn i64_is_int(v in any::<i64>()) {
        validate::<i64>(&encode(&v));
    }
}

// ---------------------------------------------------------------------------
// bool → bool
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn bool_is_bool(v in any::<bool>()) {
        validate::<bool>(&encode(&v));
    }
}

// ---------------------------------------------------------------------------
// String → text
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn string_is_text(v in any::<String>()) {
        validate::<String>(&encode(&v));
    }
}

// ---------------------------------------------------------------------------
// Vec<u8> → bytes
//
// minicbor encodes bare Vec<u8> as a CBOR array of uints, not CBOR bytes.
// The bytes encoding requires ByteVec (or cbor_with::bytes in wire types).
// We test against ByteVec here since that's what the codec produces.
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn vec_u8_is_bytes(v in proptest::collection::vec(any::<u8>(), 0..256)) {
        use minicbor::bytes::ByteVec;
        let cbor = encode(&ByteVec::from(v));
        let spec = "root = bytes\n";
        validate_cbor_bytes("root", spec, &cbor)
            .unwrap_or_else(|e| panic!("CDDL validation failed for `bytes`: {:?}", e));
    }
}

// ---------------------------------------------------------------------------
// Option<T> → T / null
//
// minicbor encodes None as CBOR null (0xf6), Some(v) as the bare value.
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn option_u64_some(v in any::<u64>()) {
        let cbor = encode(&Some(v));
        let spec = format!("root = {}\n", Option::<u64>::cddl_ref());
        validate_cbor_bytes("root", &spec, &cbor)
            .unwrap_or_else(|e| panic!("Option<u64> Some failed: {:?}", e));
    }

    #[test]
    fn option_u64_none(_v in any::<u64>()) {
        let cbor = encode(&Option::<u64>::None);
        let spec = format!("root = {}\n", Option::<u64>::cddl_ref());
        validate_cbor_bytes("root", &spec, &cbor)
            .unwrap_or_else(|e| panic!("Option<u64> None failed: {:?}", e));
    }

    #[test]
    fn option_string_some(v in any::<String>()) {
        let cbor = encode(&Some(v));
        let spec = format!("root = {}\n", Option::<String>::cddl_ref());
        validate_cbor_bytes("root", &spec, &cbor)
            .unwrap_or_else(|e| panic!("Option<String> Some failed: {:?}", e));
    }

    #[test]
    fn option_string_none(_v in any::<String>()) {
        let cbor = encode(&Option::<String>::None);
        let spec = format!("root = {}\n", Option::<String>::cddl_ref());
        validate_cbor_bytes("root", &spec, &cbor)
            .unwrap_or_else(|e| panic!("Option<String> None failed: {:?}", e));
    }
}

// ---------------------------------------------------------------------------
// cddl_ref() and cddl_definition() unit tests
// ---------------------------------------------------------------------------

#[test]
fn primitive_refs() {
    assert_eq!(u8::cddl_ref(), "uint");
    assert_eq!(u64::cddl_ref(), "uint");
    assert_eq!(i8::cddl_ref(), "int");
    assert_eq!(i64::cddl_ref(), "int");
    assert_eq!(bool::cddl_ref(), "bool");
    assert_eq!(String::cddl_ref(), "text");
    assert_eq!(Vec::<u8>::cddl_ref(), "bytes");
}

#[test]
fn primitive_definitions_are_none() {
    assert!(u64::cddl_definition().is_none());
    assert!(i64::cddl_definition().is_none());
    assert!(bool::cddl_definition().is_none());
    assert!(String::cddl_definition().is_none());
    assert!(Vec::<u8>::cddl_definition().is_none());
    assert!(Option::<u64>::cddl_definition().is_none());
}

#[test]
fn option_refs() {
    assert_eq!(Option::<u64>::cddl_ref(), "uint / null");
    assert_eq!(Option::<String>::cddl_ref(), "text / null");
}
