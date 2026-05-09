use minicbor::{Decoder, Encoder};
use std::{fmt::Display, str::FromStr};

pub fn encode<W, T, Ctx>(
    val: &T,
    e: &mut Encoder<W>,
    _ctx: &mut Ctx,
) -> Result<(), minicbor::encode::Error<W::Error>>
where
    W: minicbor::encode::Write,
    T: Display,
{
    e.str(&val.to_string())?;
    Ok(())
}

pub fn decode<'b, T, Ctx>(d: &mut Decoder<'b>, _ctx: &mut Ctx) -> Result<T, minicbor::decode::Error>
where
    T: FromStr,
    T::Err: std::fmt::Display,
{
    d.str()?
        .parse()
        .map_err(|e: T::Err| minicbor::decode::Error::message(e.to_string()))
}
