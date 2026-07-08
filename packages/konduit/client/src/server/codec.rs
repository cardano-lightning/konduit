//! ProblemDetailBody decoding is needed on every call now, so it joins the
//! closed set Codec enumerates. Error is the single, honest conversion
//! target for every Encoder/Decoder error this codec produces — see
//! `send`'s use of `<C as Codec>::Error::from(e)`.

use http_client::{Decoder, Encoder};
use konduit_wire as wire;
use problem_details::ProblemDetailBody;

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

#[cfg(feature = "json")]
mod json {
    use super::Codec;
    use http_client::JsonCodec;

    impl Codec for JsonCodec {
        type Error = serde_json::Error;
    }
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
