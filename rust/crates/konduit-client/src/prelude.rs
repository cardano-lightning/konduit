//! A prelude to use within the crate to ease imports, in particular in a multi-platform context.

#[cfg(feature = "reqwest")]
pub use http_client::reqwest::HttpClient;
#[cfg(feature = "wasm")]
pub use http_client::wasm::HttpClient;

#[cfg(feature = "reqwest")]
pub mod time {
    pub use std::time::{SystemTime, UNIX_EPOCH};
}
#[cfg(feature = "wasm")]
pub mod time {
    pub use web_time::{SystemTime, UNIX_EPOCH};
}

#[cfg(feature = "wasm")]
pub use cardano_sdk::wasm_proxy;

pub mod core {
    pub use bln_sdk::types::*;
    pub use cardano_sdk::*;
    pub use konduit_data::*;

    #[cfg(feature = "wasm")]
    pub mod wasm {
        pub use cardano_sdk::wasm::*;
        pub use konduit_data::wasm::*;
    }
}
