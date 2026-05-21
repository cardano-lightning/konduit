use crate::CborWith;
use std::marker::PhantomData;

/// Encodes `Option<T>` as either null (for `None`) or a single-element array `[x]`
/// (for `Some(x)`).
///
/// Unlike [`AsNullable`], this encoding is unambiguous under nesting —
/// `Some(None)` encodes as `[null]` and `None` encodes as `null`, so
/// `AsOptional<AsOptional<A>>` correctly roundtrips `Option<Option<T>>`.
///
/// # Wire format
///
/// This is **not** compatible with the plain CBOR convention for optional fields.
/// Use [`AsNullable`] for top-level optional struct fields where interop with other
/// CBOR implementations matters. Use [`AsOptional`] when composing with other
/// adapters or when you need `Option<Option<T>>`.
pub struct AsOptional<A>(PhantomData<A>);

impl<A, T, C> CborWith<Option<T>, C> for AsOptional<A>
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
            Some(inner) => {
                e.array(1)?;
                A::encode(inner, e, ctx)
            }
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
        d.array()?;
        Ok(Some(A::decode(d, ctx)?))
    }
}
