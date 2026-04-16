use cardano_sdk::PlutusData;
use minicbor::{Decoder, Encoder};

pub fn encode<W, C, T>(
    val: &T,
    e: &mut Encoder<W>,
    ctx: &mut C,
) -> Result<(), minicbor::encode::Error<W::Error>>
where
    W: minicbor::encode::Write,
    T: Into<PlutusData<'static>> + Clone,
    PlutusData<'static>: minicbor::Encode<C>,
{
    e.encode_with(val.clone().into(), ctx)?;
    Ok(())
}

pub fn decode<'b, C, T>(d: &mut Decoder<'b>, ctx: &mut C) -> Result<T, minicbor::decode::Error>
where
    PlutusData<'static>: minicbor::Decode<'b, C>,
    T: TryFrom<PlutusData<'static>>,
    T::Error: std::fmt::Display,
{
    let pd = d.decode_with(ctx)?;
    T::try_from(pd).map_err(|e| minicbor::decode::Error::message(e.to_string()))
}
