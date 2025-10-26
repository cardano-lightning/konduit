use cardano_tx_builder::{
    PlutusData,
    cbor::{self, ToCbor},
};
use serde::{Deserialize, Deserializer, Serializer};
use std::fmt::Display;

pub fn serialize<S, T>(field: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: Clone + Into<PlutusData<'static>>,
    PlutusData<'static>: cbor::Encode<()>,
    cbor::encode::Error<std::io::Error>: Display,
{
    let data: PlutusData<'static> = field.clone().into();
    let cbor_bytes = data.to_cbor();
    let hex_string = hex::encode(cbor_bytes);
    serializer.serialize_str(&hex_string)
}

pub fn deserialize<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: for<'a> TryFrom<&'a PlutusData<'static>>,
    for<'a> <T as TryFrom<&'a PlutusData<'static>>>::Error: Display,
    PlutusData<'static>: for<'d> cbor::Decode<'d, ()>,
    cbor::decode::Error: Display,
{
    let hex_string = String::deserialize(deserializer)?;
    let cbor_bytes = hex::decode(&hex_string).map_err(serde::de::Error::custom)?;
    let data: PlutusData<'static> =
        cbor::decode_with(cbor_bytes.as_slice(), &mut ()).map_err(serde::de::Error::custom)?;
    T::try_from(&data).map_err(serde::de::Error::custom)
}
