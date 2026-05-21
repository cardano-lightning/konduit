use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode, thiserror::Error)]
pub enum Error {
    /// No `konduit-hmac-token` header present.
    #[n(0)]
    #[error("missing konduit-hmac-token header")]
    MissingToken,
    /// Header value could not be decoded (bad base64 or bad CBOR).
    #[n(1)]
    #[error("malformed token")]
    BadToken,
    /// Token TTL has passed.
    #[n(2)]
    #[error("token expired")]
    Expired,
    /// MAC verification failed.
    #[n(3)]
    #[error("unauthorized")]
    Unauthorized,
    /// Issuance request body could not be decoded.
    #[n(4)]
    #[error("bad request")]
    BadRequest,
    /// Provided signature does not verify against the claimed keytag.
    #[n(5)]
    #[error("bad signature")]
    BadSignature,
    /// Keytag is not a recognised patron (no channel on record).
    #[n(6)]
    #[error("not a patron")]
    NotPatron,
}

impl crate::ApiError for Error {
    fn status_code(&self) -> u16 {
        match self {
            Error::BadRequest => 400,
            Error::NotPatron => 403,
            _ => 401,
        }
    }
}
