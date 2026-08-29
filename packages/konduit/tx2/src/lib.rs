const FEE_BUFFER: u64 = 4_000_000;

pub mod fuel;

//TODO:: Upstream this!
pub mod currency;
pub use currency::Currency;

pub mod network_parameters;
pub use network_parameters::NetworkParameters;

pub mod channel;
pub use channel::Channel;

pub mod staged_tx;
pub use staged_tx::StagedTx;

pub mod step;

pub mod interval;
pub use interval::Interval;

pub mod validator;
pub use validator::*;

mod cbor_box;

#[cfg(feature = "prompt")]
pub mod prompt;
