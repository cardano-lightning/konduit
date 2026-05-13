use crate::core::{Address, Credential, NetworkId, SigningKey, address::kind};
use anyhow::anyhow;
use std::cell::RefCell;

/// A rudimentary wallet interface
#[derive(Debug)]
pub struct Wallet {
    pub network_id: NetworkId,
    pub signing_key: SigningKey,
    pub stake_credential: RefCell<Option<Credential>>,
    pub exit_address: RefCell<anyhow::Result<Address<kind::Shelley>>>,
}

impl Wallet {
    pub fn new(network_id: NetworkId, signing_key: SigningKey) -> Self {
        Self {
            network_id,
            signing_key,
            stake_credential: RefCell::new(None),
            exit_address: RefCell::new(Err(anyhow!("no exit address"))),
        }
    }
}
