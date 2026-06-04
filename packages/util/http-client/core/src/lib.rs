#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

pub(crate) mod prelude;

mod transport;
pub use transport::Transport as HttpTransport;

mod request_builder;
pub use request_builder::{RequestBuilder, url};

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

#[cfg(feature = "wasm")]
mod wasm;
#[cfg(feature = "wasm")]
pub use wasm::GlooTransport;

#[cfg(feature = "wasm")]
mod wasm_client;
#[cfg(feature = "wasm")]
pub use wasm_client::WasmClient;

#[cfg(feature = "native")]
mod native;
#[cfg(feature = "native")]
pub use native::NativeTransport;
