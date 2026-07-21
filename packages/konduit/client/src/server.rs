//! Konduit server client.
//!
//! This is a very thin wrapper of http_client,
//! plumbing in the pieces from konduit-wire so that it talks to konduit-server.
//! Beyond the http client, it owns no state:
//! Tag, credentials, _etc_ are caller's responsibility
//!
pub mod codec;
pub use codec::Codec;

mod error;
pub use error::Error;

mod config;
pub use config::Config;

mod client;
pub use client::Client;
