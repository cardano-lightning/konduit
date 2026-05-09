use std::marker::PhantomData;

use crate::CborWith;

pub struct AsOption<A>(PhantomData<A>);

impl<A, T, C> CborWith<Option<T>, C> for AsOption<A>
where
    A: CborWith<T, C>,
{
    fn encode<W>(
        val: &Option<T>,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>>
    where
        W: minicbor::encode::Write,
    {
        match val {
            None => {
                e.null()?;
                Ok(())
            }
            Some(inner) => A::encode(inner, e, ctx),
        }
    }

    fn decode<'b>(
        d: &mut minicbor::Decoder<'b>,
        ctx: &mut C,
    ) -> Result<Option<T>, minicbor::decode::Error> {
        if matches!(
            d.datatype(),
            Ok(minicbor::data::Type::Null | minicbor::data::Type::Undefined)
        ) {
            d.skip()?;
            return Ok(None);
        }
        Ok(Some(A::decode(d, ctx)?))
    }
}
