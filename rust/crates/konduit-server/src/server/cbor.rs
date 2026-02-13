use cardano_tx_builder::{
    PlutusData,
    cbor::{self, ToCbor},
};
use std::convert::{From, TryFrom};
use std::fmt::{self, Display};

pub fn encode_to_cbor<T>(data: T) -> Vec<u8>
where
    T: Clone + Into<PlutusData<'static>>,
    PlutusData<'static>: ToCbor,
{
    let data: PlutusData<'static> = data.clone().into();
    data.to_cbor()
}

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
