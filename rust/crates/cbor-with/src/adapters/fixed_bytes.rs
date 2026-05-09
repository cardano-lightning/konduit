use crate::CborWith;

pub struct FixedBytes<const N: usize>(());

impl<const N: usize, T, C> CborWith<T, C> for FixedBytes<N>
where
    T: AsRef<[u8]> + From<[u8; N]>,
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
        let bytes = d.bytes()?;
        let arr: [u8; N] = bytes
            .try_into()
            .map_err(|_| minicbor::decode::Error::message(format!("expected {} bytes", N)))?;
        Ok(T::from(arr))
    }
}
