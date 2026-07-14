#[cfg(feature = "black-box-api")]
pub mod black_box_api;

pub mod wallet;

pub mod wasm;

// A prelude to use within the crate to ease imports, in particular in a multi-platform context.
pub(crate) use prelude::*;
#[allow(unused_imports)]
mod prelude {
    use http_client::{codec, transport};

    pub type HttpClient = http_client::Client<transport::Gloo, codec::Json>;
    pub fn new_http_client(url: &str) -> HttpClient {
        HttpClient::new(transport::Gloo::default(), codec::Json, url.to_string())
    }

    pub type Adaptor = konduit_client::Adaptor<transport::Gloo>;

    pub type Connector = cardano_connector_client::Connector<transport::Gloo>;

    pub mod l1 {
        #[cfg(feature = "black-box-api")]
        pub type Client<'a> = konduit_client::l1::Client<'a, super::Connector>;
    }

    pub mod l2 {
        use http_client::transport;
        #[cfg(feature = "black-box-api")]
        pub type Client<'a> = konduit_client::l2::Client<'a, transport::Gloo>;
    }

    pub mod core {
        pub use bln_sdk::types::Invoice;
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
