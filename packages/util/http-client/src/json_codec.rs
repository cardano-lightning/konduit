use crate::prelude::*;
use crate::{Decoder, Encoder};

#[derive(Debug, Clone)]
pub struct JsonCodec;

impl<T: serde::Serialize> Encoder<T> for JsonCodec {
    type Error = serde_json::Error;
    fn content_type(&self) -> &'static str {
        "application/json"
    }
    fn encode(&self, value: &T) -> Result<Vec<u8>, Self::Error> {
        serde_json::to_vec(value)
    }
}

impl<T: serde::de::DeserializeOwned> Decoder<T> for JsonCodec {
    type Error = serde_json::Error;
    fn accept_type(&self) -> &'static str {
        "application/json"
    }
    fn decode(&self, bytes: &[u8]) -> Result<T, Self::Error> {
        serde_json::from_slice(bytes)
    }
}
