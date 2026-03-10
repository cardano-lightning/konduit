use crate::wasm::Error;

/// A Result type that is convenient when operating at the boundary between Rust and JavaScript.
pub type Result<T> = std::result::Result<T, Error>;
