pub mod plutus_bytes;
pub mod plutus_list;

#[cfg(feature = "cardano_sdk")]
mod plutus_bytes_tests;
#[cfg(feature = "cardano_sdk")]
mod plutus_int_tests;
#[cfg(feature = "cardano_sdk")]
mod plutus_list_tests;

// FIXME :: To be reinstated, these tests need the cbor_with lib,
// currently abandoned
// #[cfg(feature = "cardano_sdk")]
// mod sdk_cbor {
//     use cardano_sdk::PlutusData;
//     use cbor_with::{AsNullable, AsVec, IntoTryFrom, as_module};
//     as_module!(pub mod plutus_data = IntoTryFrom<PlutusData<'static>>);
//     as_module!(pub mod nullable_plutus_data = AsNullable<IntoTryFrom<PlutusData<'static>>>);
//     as_module!(pub mod vec_plutus_data = AsVec<IntoTryFrom<PlutusData<'static>>>);
// }
// #[cfg(feature = "cardano_sdk")]
// pub use sdk_cbor::{nullable_plutus_data, plutus_data, vec_plutus_data};
