mod auth;
mod client;
mod error;
mod response;

pub mod types;

#[cfg(feature = "cli")]
pub mod cli;

pub mod blocking;
pub use auth::BasicAuth;
pub use client::Client;
pub use error::{Error, Result};
pub use response::KupoResponse;
pub use types::*;
