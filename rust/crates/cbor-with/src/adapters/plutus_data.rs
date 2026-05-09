use std::fmt::Display;

use crate::CborWith;

pub struct PlutusData(());

impl<T, C> CborWith<T, C> for PlutusData
where
    T: Clone + Into<cardano_sdk::PlutusData<'static>> + TryFrom<cardano_sdk::PlutusData<'static>>,
    T::Error: Display,
    cardano_sdk::PlutusData<'static>: minicbor::Encode<C> + for<'b> minicbor::Decode<'b, C>,
{
    fn encode<W>(
        val: &T,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>>
    where
        W: minicbor::encode::Write,
    {
        e.encode_with(val.clone().into(), ctx)?;
        Ok(())
    }

    fn decode<'b>(
        d: &mut minicbor::Decoder<'b>,
        ctx: &mut C,
    ) -> Result<T, minicbor::decode::Error> {
        let pd = d.decode_with(ctx)?;
        T::try_from(pd).map_err(|e| minicbor::decode::Error::message(e.to_string()))
    }
}
