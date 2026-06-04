#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

pub(crate) mod prelude;

pub mod url;

pub mod header_policy;
pub use header_policy::HeaderPolicy;

mod transport;
pub use transport::Transport as HttpTransport;

mod request_builder;
pub use request_builder::RequestBuilder;

mod client;
pub use client::{Client as HttpClient, ClientError};

mod codec;
pub use codec::{Decoder, Encoder};

#[cfg(feature = "json")]
mod json_codec;
#[cfg(feature = "json")]
pub use json_codec::JsonCodec;

#[cfg(feature = "cbor")]
mod cbor_codec;
#[cfg(feature = "cbor")]
pub use cbor_codec::CborCodec;

#[cfg(feature = "reqwest")]
mod reqwest_transport;
#[cfg(feature = "reqwest")]
pub use reqwest_transport::ReqwestTransport;

#[cfg(feature = "gloo")]
mod gloo_transport;
#[cfg(feature = "gloo")]
pub use gloo_transport::GlooTransport;

#[cfg(feature = "bindgen")]
pub mod bindgen;

#[cfg(all(feature = "bindgen", feature = "json"))]
pub mod bindgen_json;

#[cfg(all(feature = "bindgen", feature = "cbor"))]
pub mod bindgen_cbor;
