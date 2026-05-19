use cardano_sdk::PlutusData;
use cbor_with::{AsNullable, AsVec, IntoTryFrom, as_module};

pub mod plutus_bytes;
pub mod plutus_list;
pub mod plutus_tuple;

mod plutus_bytes_tests;
mod plutus_int_tests;
mod plutus_list_tests;

as_module!(pub mod plutus_data = IntoTryFrom<PlutusData<'static>>);
as_module!(pub mod nullable_plutus_data = AsNullable<IntoTryFrom<PlutusData<'static>>>);
as_module!(pub mod vec_plutus_data = AsVec<IntoTryFrom<PlutusData<'static>>>);
