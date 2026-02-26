//! A prelude to use within the crate to ease imports, in particular in a multi-platform context.

pub use konduit_client::wasm::Client;

pub use cardano_connector_client::{CardanoConnector, wasm::Connector};

pub mod wasm {
    pub use cardano_sdk::wasm::Result;
}

pub mod core {
    pub use cardano_connector_client::wasm::TransactionSummary;
    pub use cardano_sdk::{
        Credential, Input, NetworkId, Output, Signature, SigningKey, VerificationKey,
        address::ShelleyAddress, cbor, hash::Hash32,
    };
    pub use konduit_data::{
        Duration, SquashBody, Stage,
        wasm::{Quote, Tag},
    };
    pub use konduit_tx::{
        Bounds, ChannelOutput, NetworkParameters,
        consumer::{Intent, OpenIntent},
        filter_channels,
    };
}
