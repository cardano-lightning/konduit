use crate::prelude::*;

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
