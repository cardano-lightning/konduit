// state/keys.rs
//! Rarely-changing root configuration: which keys (if any) this process
//! embeds directly, versus delegating to an external wallet/signer. Not
//! recoverable if lost — these are the actual secrets, not settings with
//! sensible defaults.

use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, Encode, Decode)]
pub struct Keys {
    /// Set when running an embedded signer, else managed externally.
    #[n(0)]
    add_vkey: Option<[u8; 32]>,
    /// Set when running an embedded wallet, else managed externally.
    #[n(1)]
    wallet_key: Option<[u8; 32]>,
}

impl Keys {
    pub fn add_vkey(&self) -> Option<[u8; 32]> {
        self.add_vkey
    }
    pub fn set_add_vkey(&mut self, key: Option<[u8; 32]>) {
        self.add_vkey = key;
    }
    pub fn wallet_key(&self) -> Option<[u8; 32]> {
        self.wallet_key
    }
    pub fn set_wallet_key(&mut self, key: Option<[u8; 32]>) {
        self.wallet_key = key;
    }
}
