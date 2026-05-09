use std::marker::PhantomData;

use crate::CborWith;

/// Encodes `Option<T>` as either null (for `None`) or the encoded `T` (for `Some(T)`).
///
/// This is the natural encoding for optional fields in a CBOR map — a missing or null
/// field decodes as `None`, a present field decodes as `Some(T)`. It is compatible with
/// how most CBOR libraries encode optional values.
///
/// # Warning
///
/// This encoding is ambiguous under nesting. `Option<Option<T>>` cannot distinguish
/// `Some(None)` from `None` on the wire — both encode as null. If you need to compose
/// `Option` with itself or with other composers, use [`AsOptional`] instead, which
/// wraps `Some(x)` in a single-element array to make nesting unambiguous.
pub struct AsNullable<A>(PhantomData<A>);

impl<A, T, C> CborWith<Option<T>, C> for AsNullable<A>
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
