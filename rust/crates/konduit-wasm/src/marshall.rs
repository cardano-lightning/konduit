use crate::{
    core::cbor::{self, FromCbor, ToCbor},
    wasm,
};
use anyhow::anyhow;

pub(crate) trait Marshall {
    fn marshall(&self) -> String
    where
        Self: cbor::Encode<()>,
    {
        hex::encode(self.to_cbor())
    }
}

impl<A> Unmarshall for Option<A> where A: for<'d> cbor::Decode<'d, ()> {}

impl<A, B> Unmarshall for (A, B)
where
    A: for<'d> cbor::Decode<'d, ()>,
    B: for<'d> cbor::Decode<'d, ()>,
{
}

impl<A, B, C> Unmarshall for (A, B, C)
where
    A: for<'d> cbor::Decode<'d, ()>,
    B: for<'d> cbor::Decode<'d, ()>,
    C: for<'d> cbor::Decode<'d, ()>,
{
}

impl<A, B, C, D> Unmarshall for (A, B, C, D)
where
    A: for<'d> cbor::Decode<'d, ()>,
    B: for<'d> cbor::Decode<'d, ()>,
    C: for<'d> cbor::Decode<'d, ()>,
    D: for<'d> cbor::Decode<'d, ()>,
{
}

pub(crate) trait Unmarshall {
    fn unmarshall(data: &str) -> wasm::Result<Self>
    where
        Self: Sized + for<'d> cbor::Decode<'d, ()>,
    {
        let bytes =
            hex::decode(data).map_err(|e| anyhow!(e).context("malformed hex-encoded value"))?;

        let value = Self::from_cbor(&bytes[..])
            .map_err(|e| anyhow!(e).context("unable to decode from cbor"))?;

        Ok(value)
    }
}

impl<A> Marshall for Option<A> where A: cbor::Encode<()> {}

impl<A, B> Marshall for (A, B)
where
    A: cbor::Encode<()>,
    B: cbor::Encode<()>,
{
}

impl<A, B, C> Marshall for (A, B, C)
where
    A: cbor::Encode<()>,
    B: cbor::Encode<()>,
    C: cbor::Encode<()>,
{
}

impl<A, B, C, D> Marshall for (A, B, C, D)
where
    A: cbor::Encode<()>,
    B: cbor::Encode<()>,
    C: cbor::Encode<()>,
    D: cbor::Encode<()>,
{
}
