pub use crate::interface::HttpClient;
mod interface;

#[cfg(any(feature = "wasm", feature = "reqwest"))]
pub use crate::implementations::*;
mod implementations;
