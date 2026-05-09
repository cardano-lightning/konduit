use std::marker::PhantomData;

use crate::CborWith;

/// Encodes by converting `T` into `U`, decodes by converting `U` into `T`.
/// Both directions infallible.
///
/// `U` must implement minicbor's native [`minicbor::Encode`] and [`minicbor::Decode`] traits
/// directly. If `U` does not have native minicbor impls but can be encoded via another
/// [`CborWith`] adapter, use `AsIntoFrom<U, A>` instead (unimplemented).
pub struct IntoFrom<U>(PhantomData<U>);

impl<U, T, C> CborWith<T, C> for IntoFrom<U>
where
    T: Clone + Into<U> + From<U>,
    U: minicbor::Encode<C> + for<'b> minicbor::Decode<'b, C>,
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
        let u: U = d.decode_with(ctx)?;
        Ok(T::from(u))
    }
}
