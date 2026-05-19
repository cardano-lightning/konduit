#![cfg(feature = "proptest")]

use cardano_sdk::{PlutusData, cbor::ToCbor};
use minicbor::{Decode, Encode};
use proptest::prelude::*;

#[derive(Debug, PartialEq, Encode, Decode)]
#[cbor(transparent)]
struct Bytes(#[cbor(with = "crate::cbor_with::plutus_bytes", n(0))] Vec<u8>);

proptest! {
    #[test]
    fn bytes_matches_plutus(raw: Vec<u8>) {
        let tag = Bytes(raw.clone());
        let mini_bytes = minicbor::to_vec(&tag).unwrap();
        let pd_bytes = PlutusData::bytes(raw).to_cbor();
        prop_assert_eq!(mini_bytes, pd_bytes);
    }

    #[test]
    fn bytes_roundtrip(raw: Vec<u8>) {
        let tag = Bytes(raw.clone());
        let encoded = minicbor::to_vec(&tag).unwrap();
        let recovered: Bytes = minicbor::decode(&encoded).unwrap();
        prop_assert_eq!(tag, recovered);
    }

    #[test]
    fn bytes_plutus_roundtrip(raw: Vec<u8>) {
        let tag = Bytes(raw.clone());
        let pd_bytes = PlutusData::bytes(raw).to_cbor();
        let recovered: Bytes = minicbor::decode(&pd_bytes).unwrap();
        prop_assert_eq!(tag, recovered);
    }
}
