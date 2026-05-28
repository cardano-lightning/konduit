use std::marker::PhantomData;

use crate::CborWith;

pub struct AsVec<A>(PhantomData<A>);

impl<A, T, C> CborWith<Vec<T>, C> for AsVec<A>
where
    A: CborWith<T, C>,
{
    fn encode<W>(
        val: &Vec<T>,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>>
    where
        W: minicbor::encode::Write,
    {
        e.array(val.len() as u64)?;
        for item in val {
            A::encode(item, e, ctx)?;
        }
        Ok(())
    }

    fn decode<'b>(
        d: &mut minicbor::Decoder<'b>,
        ctx: &mut C,
    ) -> Result<Vec<T>, minicbor::decode::Error> {
        let len = d
            .array()?
            .ok_or_else(|| minicbor::decode::Error::message("indefinite arrays not supported"))?;
        (0..len).map(|_| A::decode(d, ctx)).collect()
    }
}
