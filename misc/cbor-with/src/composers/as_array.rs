use std::marker::PhantomData;

use crate::CborWith;

pub struct AsArray<const N: usize, A>(PhantomData<A>);

impl<const N: usize, A, T, C> CborWith<[T; N], C> for AsArray<N, A>
where
    A: CborWith<T, C>,
    T: Default + Copy, // needed to initialize before filling
{
    fn encode<W>(
        val: &[T; N],
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>>
    where
        W: minicbor::encode::Write,
    {
        e.array(N as u64)?;
        for item in val {
            A::encode(item, e, ctx)?;
        }
        Ok(())
    }

    fn decode<'b>(
        d: &mut minicbor::Decoder<'b>,
        ctx: &mut C,
    ) -> Result<[T; N], minicbor::decode::Error> {
        let mut arr = [T::default(); N];
        for slot in arr.iter_mut() {
            *slot = A::decode(d, ctx)?;
        }
        Ok(arr)
    }
}
