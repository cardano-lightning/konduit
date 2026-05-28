use crate::CborWith;

/// Encodes as a CBOR byte string. Decodes by copying into an owned buffer.
/// T must be constructible from Vec<u8> — no lifetime dependency on the decoder.
pub struct BytesOwned(());

impl<T, C> CborWith<T, C> for BytesOwned
where
    T: AsRef<[u8]> + TryFrom<Vec<u8>>,
    <T as TryFrom<Vec<u8>>>::Error: std::fmt::Display,
{
    fn encode<W>(
        val: &T,
        e: &mut minicbor::Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>>
    where
        W: minicbor::encode::Write,
    {
        e.bytes(val.as_ref())?;
        Ok(())
    }

    fn decode<'b>(
        d: &mut minicbor::Decoder<'b>,
        _ctx: &mut C,
    ) -> Result<T, minicbor::decode::Error> {
        let slice = d.bytes()?;
        T::try_from(slice.to_vec()).map_err(|e| minicbor::decode::Error::message(e.to_string()))
    }
}
