//! A prelude to use within the crate to ease imports, in particular in a multi-platform context.

#[cfg(feature = "wasm")]
pub use cardano_sdk::wasm_proxy;

pub mod core {
    pub use bln_sdk::types::*;
    pub use cardano_sdk::*;
    pub use konduit_data::*;
    pub use konduit_tx::*;

    #[cfg(feature = "wasm")]
    pub mod wasm {
        pub use cardano_sdk::wasm::*;
        pub use konduit_data::wasm::*;
    }
}
