// tests/integration.rs

use cbor_with::{AsNullable, AsOptional, AsVec, DisplayFromStr, as_module};
use minicbor::{Decode, Encode};
use std::net::IpAddr;

// ---- helpers ----

fn roundtrip<T>(val: &T) -> T
where
    T: minicbor::Encode<()> + for<'b> minicbor::Decode<'b, ()>,
{
    let bytes = minicbor::to_vec(val).unwrap();
    minicbor::decode(&bytes).unwrap()
}

fn roundtrip_bytes<T>(val: &T) -> Vec<u8>
where
    T: minicbor::Encode<()>,
{
    minicbor::to_vec(val).unwrap()
}

// ---- test types ----

#[derive(Debug, PartialEq, Encode, Decode)]
struct WithDisplayFromStr {
    #[cbor(n(0), with = "cbor_with::display_from_str")]
    addr: IpAddr,
}

#[derive(Debug, PartialEq, Encode, Decode)]
struct WithNulDisplayFromStr {
    #[cbor(n(0), with = "cbor_with::nullable_display_from_str")]
    addr: Option<IpAddr>,
}

#[derive(Debug, PartialEq, Encode, Decode)]
struct WithVecDisplayFromStr {
    #[cbor(n(0), with = "cbor_with::vec_display_from_str")]
    addrs: Vec<IpAddr>,
}

as_module!(mod opt_opt_addr = AsOptional<AsOptional<DisplayFromStr>>);

#[derive(Debug, PartialEq, Encode, Decode)]
struct WithOptOptDisplayFromStr {
    #[cbor(n(0), with = "opt_opt_addr")]
    addr: Option<Option<IpAddr>>,
}

as_module!(mod vec_opt_addr = AsVec<AsNullable<DisplayFromStr>>);

#[derive(Debug, PartialEq, Encode, Decode)]
struct WithVecOptDisplayFromStr {
    #[cbor(n(0), with = "vec_opt_addr")]
    addrs: Vec<Option<IpAddr>>,
}

#[derive(Debug, PartialEq, Encode, Decode)]
struct WithBytesOwned {
    #[cbor(n(0), with = "cbor_with::bytes_owned")]
    data: Vec<u8>,
}

#[derive(Debug, PartialEq, Encode, Decode)]
struct WithFixedBytes {
    #[cbor(n(0), with = "cbor_with::fixed_bytes_32")]
    key: [u8; 32],
}

#[derive(Debug, PartialEq, Encode, Decode)]
struct WithSame {
    #[cbor(n(0), with = "cbor_with::same")]
    inner: Inner,
}

#[derive(Debug, PartialEq, Encode, Decode)]
struct Inner {
    #[cbor(n(0))]
    value: u32,
}

// ---- tests ----

#[test]
fn test_display_from_str_roundtrip() {
    let val = WithDisplayFromStr {
        addr: "127.0.0.1".parse().unwrap(),
    };
    assert_eq!(val, roundtrip(&val));
}

#[test]
fn test_display_from_str_encodes_as_string() {
    let val = WithDisplayFromStr {
        addr: "127.0.0.1".parse().unwrap(),
    };
    let bytes = roundtrip_bytes(&val);
    assert!(bytes.windows(9).any(|w| w == b"127.0.0.1"));
}

#[test]
fn test_option_some_roundtrip() {
    let val = WithNulDisplayFromStr {
        addr: Some("::1".parse().unwrap()),
    };
    assert_eq!(val, roundtrip(&val));
}

#[test]
fn test_option_none_roundtrip() {
    let val = WithNulDisplayFromStr { addr: None };
    assert_eq!(val, roundtrip(&val));
}

#[test]
fn test_option_none_encodes_as_null() {
    let mut buf = Vec::new();
    let mut e = minicbor::Encoder::new(&mut buf);
    cbor_with::nullable_display_from_str::encode::<_, (), Option<IpAddr>>(&None, &mut e, &mut ())
        .unwrap();
    assert_eq!(buf, &[0xf6]);
}

#[test]
fn test_vec_roundtrip() {
    let val = WithVecDisplayFromStr {
        addrs: vec!["127.0.0.1".parse().unwrap(), "::1".parse().unwrap()],
    };
    assert_eq!(val, roundtrip(&val));
}

#[test]
fn test_vec_empty_roundtrip() {
    let val = WithVecDisplayFromStr { addrs: vec![] };
    assert_eq!(val, roundtrip(&val));
}

#[test]
fn test_opt_opt_some_some_roundtrip() {
    let val = WithOptOptDisplayFromStr {
        addr: Some(Some("10.0.0.1".parse().unwrap())),
    };
    assert_eq!(val, roundtrip(&val));
}

#[test]
fn test_opt_opt_some_none_roundtrip() {
    let val = WithOptOptDisplayFromStr { addr: Some(None) };
    assert_eq!(val, roundtrip(&val));
}

#[test]
fn test_opt_opt_none_roundtrip() {
    let val = WithOptOptDisplayFromStr { addr: None };
    assert_eq!(val, roundtrip(&val));
}

#[test]
fn test_vec_opt_roundtrip() {
    let val = WithVecOptDisplayFromStr {
        addrs: vec![
            Some("127.0.0.1".parse().unwrap()),
            None,
            Some("::1".parse().unwrap()),
        ],
    };
    assert_eq!(val, roundtrip(&val));
}

#[test]
fn test_bytes_owned_roundtrip() {
    let val = WithBytesOwned {
        data: vec![1, 2, 3, 4, 5],
    };
    assert_eq!(val, roundtrip(&val));
}

#[test]
fn test_bytes_owned_empty_roundtrip() {
    let val = WithBytesOwned { data: vec![] };
    assert_eq!(val, roundtrip(&val));
}

#[test]
fn test_fixed_bytes_roundtrip() {
    let val = WithFixedBytes { key: [0xab; 32] };
    assert_eq!(val, roundtrip(&val));
}

#[test]
fn test_same_roundtrip() {
    let val = WithSame {
        inner: Inner { value: 42 },
    };
    assert_eq!(val, roundtrip(&val));
}
