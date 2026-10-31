use konduit_data::VerifyingKey;
use serde::{Deserialize, Serialize};

use crate::{channels, ctx, paymes, signer};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub params: ctx::Params,
    pub signer: signer::Config,
    pub channels: channels::Config,
    pub paymes: paymes::Config,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("expect peer: {0:?}")]
    ExpectPeer(VerifyingKey),
}

impl Config {
    pub fn verify(&self) -> Result<(), Error> {
        Ok(())
    }
}
