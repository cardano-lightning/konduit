#![cfg(feature = "proptest")]
#![allow(unused_imports, dead_code)]

use cardano_sdk::{PlutusData, cbor::ToCbor};
use proptest::prelude::*;

proptest! {
    #[test]
    fn u8_matches_plutus(val: u8) {
        let mini = minicbor::to_vec(&val).unwrap();
        let pd = PlutusData::integer(val as i128).to_cbor();
        prop_assert_eq!(mini, pd);
    }

    #[test]
    fn u16_matches_plutus(val: u16) {
        let mini = minicbor::to_vec(&val).unwrap();
        let pd = PlutusData::integer(val as i128).to_cbor();
        prop_assert_eq!(mini, pd);
    }

    #[test]
    fn u32_matches_plutus(val: u32) {
        let mini = minicbor::to_vec(&val).unwrap();
        let pd = PlutusData::integer(val as i128).to_cbor();
        prop_assert_eq!(mini, pd);
    }

    #[test]
    fn u64_matches_plutus(val: u64) {
        let mini = minicbor::to_vec(&val).unwrap();
        let pd = PlutusData::integer(val as i128).to_cbor();
        prop_assert_eq!(mini, pd);
    }

    #[test]
    fn i8_matches_plutus(val: i8) {
        let mini = minicbor::to_vec(&val).unwrap();
        let pd = PlutusData::integer(val as i128).to_cbor();
        prop_assert_eq!(mini, pd);
    }

    #[test]
    fn i16_matches_plutus(val: i16) {
        let mini = minicbor::to_vec(&val).unwrap();
        let pd = PlutusData::integer(val as i128).to_cbor();
        prop_assert_eq!(mini, pd);
    }

    #[test]
    fn i32_matches_plutus(val: i32) {
        let mini = minicbor::to_vec(&val).unwrap();
        let pd = PlutusData::integer(val as i128).to_cbor();
        prop_assert_eq!(mini, pd);
    }

    #[test]
    fn i64_matches_plutus(val: i64) {
        let mini = minicbor::to_vec(&val).unwrap();
        let pd = PlutusData::integer(val as i128).to_cbor();
        prop_assert_eq!(mini, pd);
    }
}
