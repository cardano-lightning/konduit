#![cfg(feature = "proptest")]

use cardano_sdk::{PlutusData, cbor::ToCbor};
use minicbor::{Decode, Encode};
use proptest::prelude::*;

#[derive(Debug, PartialEq, Encode, Decode)]
#[cbor(transparent)]
struct List(#[cbor(with = "crate::cbor_with::plutus_list", n(0))] Vec<i64>);

proptest! {
    #[test]
    fn list_matches_plutus(raw: Vec<i64>) {
        let l = List(raw.clone());
        let mini_bytes = minicbor::to_vec(&l).unwrap();
        let pd_bytes = PlutusData::list(
            raw.iter().map(|x| PlutusData::integer(*x)
            )).to_cbor();
        prop_assert_eq!(mini_bytes, pd_bytes);
    }

    #[test]
    fn list_roundtrip(raw: Vec<i64>) {
        let l = List(raw);
        let encoded = minicbor::to_vec(&l).unwrap();
        let recovered: List = minicbor::decode(&encoded).unwrap();
        prop_assert_eq!(l, recovered);
    }

    #[test]
    fn list_plutus_roundtrip(raw: Vec<i64>) {
        let l = List(raw.clone());
        let pd_bytes = PlutusData::list(
            raw.iter().map(|x| PlutusData::integer(*x))
        ).to_cbor();
        let recovered: List = minicbor::decode(&pd_bytes).unwrap();
        prop_assert_eq!(l, recovered);
    }
}
