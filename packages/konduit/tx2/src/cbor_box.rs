use minicbor::{
    Decode, Decoder, Encode, Encoder,
    decode::Error as DecodeError,
    encode::{Error as EncodeError, Write},
};

pub fn encode<T, C, W>(
    value: &T,
    e: &mut Encoder<W>,
    ctx: &mut C,
) -> Result<(), EncodeError<W::Error>>
where
    T: Encode<C>,
    W: Write,
{
    value.encode(e, ctx)
}

pub fn decode<'b, T, C>(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Box<T>, DecodeError>
where
    T: Decode<'b, C>,
{
    T::decode(d, ctx).map(Box::new)
}
