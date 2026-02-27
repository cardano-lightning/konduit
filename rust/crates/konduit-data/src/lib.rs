pub const MAX_UNSQUASHED: usize = 10;
pub const MAX_EXCLUDE_LENGTH: usize = 10;

mod adaptor_info;
mod base;
mod channel_parameters;
mod cheque;
mod cheque_body;
mod constants;
mod datum;
mod indexes;
mod l1_channel;
mod locked;
mod pay_body;
mod pending;
mod quote;
mod quote_body;
mod receipt;
mod redeemer;
mod squash;
mod squash_body;
mod squash_proposal;
mod squash_status;
mod stage;
mod unlocked;
mod used;
mod utils;

pub use adaptor_info::*;
pub use base::*;
pub use channel_parameters::*;
pub use cheque::*;
pub use cheque_body::*;
pub use constants::*;
pub use datum::*;
pub use indexes::*;
pub use l1_channel::*;
pub use locked::*;
pub use pay_body::*;
pub use pending::*;
pub use quote::*;
pub use quote_body::*;
pub use receipt::*;
pub use redeemer::*;
pub use squash::*;
pub use squash_body::*;
pub use squash_proposal::*;
pub use squash_status::*;
pub use stage::*;
pub use unlocked::*;
pub use used::*;

#[cfg(feature = "wasm")]
pub mod wasm {
    use super::{adaptor_info, base, quote};
    pub use adaptor_info::wasm::*;
    pub use base::wasm::*;
    pub use quote::wasm::*;
}
