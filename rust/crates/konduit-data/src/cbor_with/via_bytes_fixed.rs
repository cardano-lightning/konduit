use minicbor::{Decoder, Encoder};

///FIXME :: Preferably we we'd use AsRef<[u8;N]> but we don't actually have this
/// for types we care about like `VerificationKey`
/// Similar story for the case where we have `[u8;N]: From<&T>`
pub fn encode<const N: usize, W, C, T>(
    val: &T,
    e: &mut Encoder<W>,
    _ctx: &mut C,
) -> Result<(), minicbor::encode::Error<W::Error>>
where
    W: minicbor::encode::Write,
    [u8; N]: From<T>,
    T: Clone,
{
    e.bytes(&<[u8; N]>::from(val.clone()))?;
    Ok(())
}

pub fn decode<'b, const N: usize, C, T>(
    d: &mut Decoder<'b>,
    _ctx: &mut C,
) -> Result<T, minicbor::decode::Error>
where
    T: From<[u8; N]>,
{
    let bytes = d.bytes()?;
    let arr: [u8; N] = bytes
        .try_into()
        .map_err(|_| minicbor::decode::Error::message(format!("expected {} bytes", N)))?;
    Ok(T::from(arr))
}
