macro_rules! impl_cbor_encode {
    ($t:ty, $($field:ident),+) => {
        impl<C> minicbor::Encode<C> for $t {
            fn encode<W: minicbor::encode::Write>(
                &self,
                e: &mut minicbor::Encoder<W>,
                ctx: &mut C,
            ) -> Result<(), minicbor::encode::Error<W::Error>> {
                e.begin_array()?;
                $(e.encode_with(&self.$field, ctx)?;)+
                e.end()?;
                Ok(())
            }
        }
    };
}

macro_rules! impl_cbor_decode {
    ($t:ty, $($field:ident),+) => {
        impl<'b, C> minicbor::Decode<'b, C> for $t {
            fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
                d.array()?;
                $(let $field = d.decode_with(ctx)?;)+
                $crate::cbor_with::constr::end_array(d)?;
                Ok(Self { $($field,)+ _marker: std::marker::PhantomData })
            }
        }
    };
}
