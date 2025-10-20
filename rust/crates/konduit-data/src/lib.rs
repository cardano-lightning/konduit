use cardano_tx_builder::{PlutusData, Signature};

mod base;
pub use base::*;

mod datum;
pub use datum::*;

mod cheque;
pub use cheque::*;

mod cheque_body;
pub use cheque_body::*;

mod constants;
pub use constants::*;

mod evidence;
pub use evidence::*;

mod mixed_cheque;
pub use mixed_cheque::*;

mod mixed_cheques;
pub use mixed_cheques::*;

mod mixed_receipt;
pub use mixed_receipt::*;

mod pending;
pub use pending::*;

mod pendings;
pub use pendings::*;

mod receipt;
pub use receipt::*;

mod redeemer;
pub use redeemer::*;

mod stage;
pub use stage::*;

mod step;
pub use step::*;

mod steps;
pub use steps::*;

mod squash;
pub use squash::*;

mod squash_body;
pub use squash_body::*;

mod unlocked;
pub use unlocked::*;

mod unlockeds;
pub use unlockeds::*;

mod unpends;
pub use unpends::*;

pub const MAX_TAG_LENGTH: usize = 32;
pub const MAX_EXCLUDE_LENGTH: usize = 30;
pub const MAX_UNSQUASHED: usize = 30;

pub fn signature_to_plutus_data(signature: Signature) -> PlutusData<'static> {
    PlutusData::from(&<[u8; 64]>::from(signature))
}

fn signature_from_plutus_data(plutus_data: &PlutusData) -> anyhow::Result<Signature> {
    Ok(Signature::from(<&[u8; 64]>::try_from(plutus_data)?.clone()))
}
