use cardano_tx_builder::{
    PlutusData,
    cbor::{self},
};
use std::{
    convert::{From, TryFrom},
    fmt::{self, Display},
};

#[derive(Debug)]
pub enum DecodeCborError<TError> {
    Cbor(cbor::decode::Error),
    Type(TError),
}

impl<TError> From<cbor::decode::Error> for DecodeCborError<TError> {
    fn from(e: cbor::decode::Error) -> Self {
        DecodeCborError::Cbor(e)
    }
}

impl<TError: Display> Display for DecodeCborError<TError> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DecodeCborError::Cbor(e) => write!(f, "CBOR decode error: {}", e),
            DecodeCborError::Type(e) => write!(f, "Type conversion error: {}", e),
        }
    }
}

pub fn decode_from_cbor<'b, T>(
    cbor_bytes: &'b [u8],
) -> Result<T, DecodeCborError<<T as TryFrom<PlutusData<'static>>>::Error>>
where
    T: TryFrom<PlutusData<'static>> + 'static,
    PlutusData<'static>: for<'d> cbor::Decode<'d, ()>,
    <T as TryFrom<PlutusData<'static>>>::Error: Display,
{
    let data: PlutusData<'static> = cbor::decode(cbor_bytes).map_err(DecodeCborError::Cbor)?;
    T::try_from(data).map_err(DecodeCborError::Type)
}
