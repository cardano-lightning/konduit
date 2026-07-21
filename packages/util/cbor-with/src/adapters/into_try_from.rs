use std::marker::PhantomData;

use crate::CborWith;

/// Encodes by converting `T` into `U`, decodes by trying to convert `U` into `T`.
/// Encode infallible, decode fallible.
///
/// `U` must implement minicbor's native [`minicbor::Encode`] and [`minicbor::Decode`] traits
/// directly. If `U` does not have native minicbor impls but can be encoded via another
/// [`CborWith`] adapter, use `AsIntoTryFrom<U, A>` instead (unimplemented).
pub struct IntoTryFrom<U>(PhantomData<U>);

impl<U, T, C> CborWith<T, C> for IntoTryFrom<U>
where
    T: Clone + Into<U> + TryFrom<U>,
    <T as TryFrom<U>>::Error: std::fmt::Display,
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
        T::try_from(u).map_err(|e| minicbor::decode::Error::message(e.to_string()))
    }
}
