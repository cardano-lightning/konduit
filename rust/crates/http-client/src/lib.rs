pub use crate::interface::HttpClient;
mod interface;

#[cfg(feature = "wasm")]
pub use crate::implementations::*;
mod implementations;
