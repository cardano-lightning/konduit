use crate::CborWith;

/// Encodes as a CBOR byte string. Decodes by borrowing from the input buffer.
/// T must be constructible from &'b [u8] — the decoded lifetime is tied to the decoder.
pub struct Bytes(());

impl<T, C> CborWith<T, C> for Bytes
where
    T: AsRef<[u8]> + for<'r> TryFrom<&'r [u8]>,
    for<'r> <T as TryFrom<&'r [u8]>>::Error: std::fmt::Display,
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
        T::try_from(slice).map_err(|e| minicbor::decode::Error::message(e.to_string()))
    }
}
