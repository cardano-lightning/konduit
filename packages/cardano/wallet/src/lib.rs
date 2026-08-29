mod embedded;
pub use embedded::{Config, Embedded, Error as EmbeddedError};

pub mod txs;

mod wallet;
pub use wallet::Wallet;

#[cfg(feature = "cli")]
pub mod cmd;
#[cfg(feature = "cli")]
pub use cmd::Cmd;

#[cfg(any(target_arch = "wasm32", feature = "cli"))]
mod cbor;

#[cfg(target_arch = "wasm32")]
mod cip30;

#[cfg(all(target_arch = "wasm32", feature = "web"))]
mod web;

#[cfg(target_arch = "wasm32")]
pub use cip30::{Cip30, Error as Cip30Error};
