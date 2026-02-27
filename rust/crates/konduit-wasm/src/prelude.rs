//! A prelude to use within the crate to ease imports, in particular in a multi-platform context.

pub use konduit_client::wasm::Client;

pub use cardano_connector_client::{CardanoConnector, wasm::Connector};

pub use cardano_sdk::wasm::Result;

pub mod core {
    pub use cardano_sdk::*;
    pub use konduit_data::*;
    pub use konduit_tx::*;

    pub mod wasm {
        pub use cardano_connector_client::types::wasm::*;
        pub use cardano_sdk::wasm::*;
        pub use konduit_data::wasm::*;
    }
}
