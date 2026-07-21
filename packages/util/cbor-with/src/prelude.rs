#[macro_export]
macro_rules! as_module {
    ($vis:vis mod $name:ident = $adapter:ty) => {
        $vis mod $name {
            use super::*;

            pub fn encode<W, C, T>(
                val: &T,
                e: &mut minicbor::Encoder<W>,
                ctx: &mut C,
            ) -> Result<(), minicbor::encode::Error<W::Error>>
            where
                W: minicbor::encode::Write,
                $adapter: $crate::CborWith<T, C>,
            {
                <$adapter as $crate::CborWith<T, C>>::encode(val, e, ctx)
            }

            pub fn decode<'b, C, T>(
                d: &mut minicbor::Decoder<'b>,
                ctx: &mut C,
            ) -> Result<T, minicbor::decode::Error>
            where
                $adapter: $crate::CborWith<T, C>,
            {
                <$adapter as $crate::CborWith<T, C>>::decode(d, ctx)
            }
        }
    };
}
