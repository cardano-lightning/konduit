use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Encode, Decode)]
pub enum MediaFormat {
    #[n(0)]
    Cbor,
    #[n(1)]
    Json,
}

impl MediaFormat {
    /// Returns the corresponding MIME type string.
    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Json => "application/json",
            Self::Cbor => "application/cbor",
        }
    }
}
