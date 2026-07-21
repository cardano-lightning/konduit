use http_client::{Decoder, Encoder};
use konduit_wire as wire;
use minicbor::{Decode, Encode};
use problem_details::ProblemDetailBody;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub trait Codec:
    Encoder<()>
    + Decoder<wire::info::Response>
    + Encoder<wire::reg::cobbl3::Body>
    + Decoder<wire::reg::cobbl3::Response>
    + Encoder<wire::auth::squash::Body>
    + Decoder<wire::auth::squash::Response>
    + Decoder<wire::auth::state::Response>
    + Encoder<wire::auth::pay::bolt11::quote::Body>
    + Decoder<wire::auth::pay::bolt11::quote::Response>
    + Encoder<wire::auth::pay::bolt11::commit::Body>
    + Decoder<wire::auth::pay::bolt11::commit::Response>
    + Decoder<ProblemDetailBody>
{
    type Error: std::error::Error
        + Send
        + Sync
        + 'static
        + From<<Self as Encoder<()>>::Error>
        + From<<Self as Decoder<wire::info::Response>>::Error>
        + From<<Self as Encoder<wire::reg::cobbl3::Body>>::Error>
        + From<<Self as Decoder<wire::reg::cobbl3::Response>>::Error>
        + From<<Self as Encoder<wire::auth::squash::Body>>::Error>
        + From<<Self as Decoder<wire::auth::squash::Response>>::Error>
        + From<<Self as Decoder<wire::auth::state::Response>>::Error>
        + From<<Self as Encoder<wire::auth::pay::bolt11::quote::Body>>::Error>
        + From<<Self as Decoder<wire::auth::pay::bolt11::quote::Response>>::Error>
        + From<<Self as Encoder<wire::auth::pay::bolt11::commit::Body>>::Error>
        + From<<Self as Decoder<wire::auth::pay::bolt11::commit::Response>>::Error>
        + From<<Self as Decoder<ProblemDetailBody>>::Error>;
}

#[cfg(feature = "cbor")]
mod cbor {
    use super::Codec;
    use http_client::CborCodec;

    #[derive(Debug, thiserror::Error)]
    pub enum CborCodecError {
        #[error("cbor encode error: {0}")]
        Encode(#[from] minicbor::encode::Error<core::convert::Infallible>),
        #[error("cbor decode error: {0}")]
        Decode(#[from] minicbor::decode::Error),
    }

    impl Codec for CborCodec {
        type Error = CborCodecError;
    }
}

#[cfg(feature = "cbor")]
pub use cbor::CborCodecError;

#[cfg(feature = "json")]
mod json {
    use super::Codec;
    use http_client::JsonCodec;

    impl Codec for JsonCodec {
        type Error = serde_json::Error;
    }
}

/// Which concrete codec a `Config` selects. A closed, known set — every
/// real construction path goes through `Client::from_config`, and nothing
/// in this codebase ever supplies a caller-defined codec, so there's no
/// value chasing an open `C: Codec` type parameter through `Client`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(rename_all = "lowercase")
)]
pub enum Kind {
    #[cfg(feature = "cbor")]
    #[n(0)]
    Cbor,
    #[cfg(feature = "json")]
    #[n(1)]
    Json,
}

impl std::fmt::Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            #[cfg(feature = "cbor")]
            Kind::Cbor => "cbor",
            #[cfg(feature = "json")]
            Kind::Json => "json",
        };
        write!(f, "{s}")
    }
}

impl std::str::FromStr for Kind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            #[cfg(feature = "cbor")]
            "cbor" => Ok(Kind::Cbor),
            #[cfg(feature = "json")]
            "json" => Ok(Kind::Json),
            other => Err(format!(
                "unknown codec kind `{other}` (expected one of: cbor, json)"
            )),
        }
    }
}

/// Unifies the underlying codecs' errors into one type for `Any`.
#[derive(Debug, thiserror::Error)]
pub enum AnyError {
    #[cfg(feature = "cbor")]
    #[error(transparent)]
    Cbor(#[from] CborCodecError),
    #[cfg(feature = "json")]
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// `Any` is `Codec`'s runtime-chosen implementation: `Client<T, C>` stays
/// generic over `C: Codec` exactly as originally written; `Any` is just
/// another type implementing that trait, delegating to whichever variant
/// `Kind` selected at construction.
pub enum Any {
    #[cfg(feature = "cbor")]
    Cbor(http_client::CborCodec),
    #[cfg(feature = "json")]
    Json(http_client::JsonCodec),
}

impl From<Kind> for Any {
    fn from(kind: Kind) -> Self {
        match kind {
            #[cfg(feature = "cbor")]
            Kind::Cbor => Any::Cbor(http_client::CborCodec),
            #[cfg(feature = "json")]
            Kind::Json => Any::Json(http_client::JsonCodec),
        }
    }
}

#[cfg(all(feature = "json", feature = "cbor"))]
impl<T> Encoder<T> for Any
where
    T: Serialize + Encode<()>,
{
    type Error = AnyError;

    fn content_type(&self) -> &'static str {
        match self {
            Any::Cbor(c) => Encoder::<T>::content_type(c),
            Any::Json(c) => Encoder::<T>::content_type(c),
        }
    }

    fn encode(&self, value: &T) -> Result<Vec<u8>, Self::Error> {
        match self {
            Any::Cbor(c) => Ok(c.encode(value).map_err(CborCodecError::from)?),
            Any::Json(c) => Ok(c.encode(value)?),
        }
    }
}

#[cfg(all(feature = "json", feature = "cbor"))]
impl<T> Decoder<T> for Any
where
    T: for<'b> Decode<'b, ()> + serde::de::DeserializeOwned,
{
    type Error = AnyError;

    fn accept_type(&self) -> &'static str {
        match self {
            Any::Cbor(c) => Decoder::<T>::accept_type(c),
            Any::Json(c) => Decoder::<T>::accept_type(c),
        }
    }

    fn decode(&self, bytes: &[u8]) -> Result<T, Self::Error> {
        match self {
            Any::Cbor(c) => Ok(c.decode(bytes).map_err(CborCodecError::from)?),
            Any::Json(c) => Ok(c.decode(bytes)?),
        }
    }
}

#[cfg(all(feature = "cbor", not(feature = "json")))]
impl<T: Encode<()>> Encoder<T> for Any {
    type Error = AnyError;
    fn content_type(&self) -> &'static str {
        let Any::Cbor(c) = self;
        Encoder::<T>::content_type(c)
    }
    fn encode(&self, value: &T) -> Result<Vec<u8>, Self::Error> {
        let Any::Cbor(c) = self;
        Ok(c.encode(value).map_err(CborCodecError::from)?)
    }
}

#[cfg(all(feature = "cbor", not(feature = "json")))]
impl<T: for<'b> Decode<'b, ()>> Decoder<T> for Any {
    type Error = AnyError;
    fn accept_type(&self) -> &'static str {
        let Any::Cbor(c) = self;
        Decoder::<T>::accept_type(c)
    }
    fn decode(&self, bytes: &[u8]) -> Result<T, Self::Error> {
        let Any::Cbor(c) = self;
        Ok(c.decode(bytes).map_err(CborCodecError::from)?)
    }
}

#[cfg(all(feature = "json", not(feature = "cbor")))]
impl<T: Serialize> Encoder<T> for Any {
    type Error = AnyError;
    fn content_type(&self) -> &'static str {
        let Any::Json(c) = self;
        Encoder::<T>::content_type(c)
    }
    fn encode(&self, value: &T) -> Result<Vec<u8>, Self::Error> {
        let Any::Json(c) = self;
        Ok(c.encode(value)?)
    }
}

#[cfg(all(feature = "json", not(feature = "cbor")))]
impl<T: serde::de::DeserializeOwned> Decoder<T> for Any {
    type Error = AnyError;
    fn accept_type(&self) -> &'static str {
        let Any::Json(c) = self;
        Decoder::<T>::accept_type(c)
    }
    fn decode(&self, bytes: &[u8]) -> Result<T, Self::Error> {
        let Any::Json(c) = self;
        Ok(c.decode(bytes)?)
    }
}

#[cfg(not(any(feature = "cbor", feature = "json")))]
compile_error!(
    "konduit-client requires at least one of the `cbor` or `json` features \
     (Codec::Any needs at least one wire format to implement Encoder/Decoder)."
);

impl Codec for Any {
    type Error = AnyError;
}
