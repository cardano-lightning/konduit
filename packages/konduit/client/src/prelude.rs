//! A prelude to use within the crate to ease imports, in particular in a multi-platform context.

pub mod core {
    pub use cardano_sdk::{
        Address, Credential, Hash, Input, NetworkId, Output, SigningKey, address,
    };
    pub use konduit_core::{ChequeBody, Duration, Keytag, Lock, Locked, Squash, SquashBody, Tag};
    pub use konduit_tx::{Bounds, Channel, KONDUIT_VALIDATOR, NetworkParameters, consumer};
}
