use cardano_tx_builder::{PlutusData, Signature};

pub mod cheque;
pub mod cheque_body;
pub mod constants;
pub mod evidence;
pub mod receipt;
pub mod squash;
pub mod squash_body;
pub mod unlocked;

pub const MAX_TAG_LENGTH: usize = 32;
pub const MAX_EXCLUDE_LENGTH: usize = 30;
pub const MAX_UNLOCKEDS_LENGTH: usize = 30;

pub fn signature_to_plutus_data(signature: Signature) -> PlutusData<'static> {
    PlutusData::from(&<[u8; 64]>::from(signature))
}

fn signature_from_plutus_data(plutus_data: &PlutusData) -> anyhow::Result<Signature> {
    Ok(Signature::from(<&[u8; 64]>::try_from(plutus_data)?.clone()))
}
