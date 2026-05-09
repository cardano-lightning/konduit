use crate::CborWith;

/// Encodes and decodes `T` using its own native minicbor [`minicbor::Encode`] and
/// [`minicbor::Decode`] impls. Useful as a base case for composers —
/// e.g. `AsVec<Same>` for a `Vec<T>` where `T` already derives Encode/Decode.
pub struct Same(());

impl<T, C> CborWith<T, C> for Same
where
    T: minicbor::Encode<C> + for<'b> minicbor::Decode<'b, C>,
{
    fn encode<W>(
        val: &T,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>>
    where
        W: minicbor::encode::Write,
    {
        e.encode_with(val, ctx)?;
        Ok(())
    }

    fn decode<'b>(
        d: &mut minicbor::Decoder<'b>,
        ctx: &mut C,
    ) -> Result<T, minicbor::decode::Error> {
        d.decode_with(ctx)
    }
}
