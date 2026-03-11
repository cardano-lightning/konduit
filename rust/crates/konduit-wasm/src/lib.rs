#[cfg(feature = "black-box-api")]
pub mod black_box_api;

pub mod wasm;

// A prelude to use within the crate to ease imports, in particular in a multi-platform context.
pub(crate) use prelude::*;
#[allow(unused_imports)]
mod prelude {
    pub use cardano_connector::CardanoConnector;
    pub use cardano_connector_client::Connector;
    pub use http_client_wasm::HttpClient;
    pub use konduit_client::{Adaptor, l1, l2};
    pub mod core {
        pub use cardano_connector_client::types::*;
        pub use cardano_sdk::*;
        pub use konduit_data::*;
        pub use konduit_tx::*;
        // NOTE: 'funny enough', #[wasm_bindgen] explicitly uses core::borrow for some of the
        // automatic derivations... which means that if we override core, we run into funny
        // problems.
        pub use std::borrow;
    }
}
