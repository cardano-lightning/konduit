mod adaptor;
pub use adaptor::Adaptor;

#[cfg(feature = "cli")]
pub mod cli;

mod l2;
pub use l2::Client;

mod prelude;
pub(crate) use prelude::*;

#[cfg(feature = "wasm")]
pub mod wasm {
    use super::*;
    pub use adaptor::wasm::Adaptor;
    pub use l2::wasm::Client;
}
