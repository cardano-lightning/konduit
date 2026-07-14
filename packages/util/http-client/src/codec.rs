#[cfg(feature = "json")]
mod json;
#[cfg(feature = "json")]
pub use json::JsonCodec as Json;

#[cfg(feature = "cbor")]
mod cbor;
#[cfg(feature = "cbor")]
pub use cbor::CborCodec as Cbor;

pub trait Encoder<T> {
    type Error: core::error::Error + Send + Sync + 'static;
    fn content_type(&self) -> &'static str;
    fn encode(&self, value: &T) -> Result<Vec<u8>, Self::Error>;
}

pub trait Decoder<T> {
    type Error: core::error::Error + Send + Sync + 'static;
    fn accept_type(&self) -> &'static str;
    fn decode(&self, bytes: &[u8]) -> Result<T, Self::Error>;
}
