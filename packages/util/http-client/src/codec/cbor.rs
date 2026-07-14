use crate::{Decoder, Encoder};

pub struct CborCodec;

impl<T: minicbor::Encode<()>> Encoder<T> for CborCodec {
    type Error = minicbor::encode::Error<core::convert::Infallible>;

    fn content_type(&self) -> &'static str {
        "application/cbor"
    }

    fn encode(&self, value: &T) -> Result<Vec<u8>, Self::Error> {
        let mut buf = Vec::new();
        let mut enc = minicbor::Encoder::new(&mut buf);
        value.encode(&mut enc, &mut ())?;
        Ok(buf)
    }
}

impl<T: for<'b> minicbor::Decode<'b, ()>> Decoder<T> for CborCodec {
    type Error = minicbor::decode::Error;

    fn accept_type(&self) -> &'static str {
        "application/cbor"
    }

    fn decode(&self, bytes: &[u8]) -> Result<T, Self::Error> {
        minicbor::decode(bytes)
    }
}
