//! Top-level state managing l1(s) and l2s. Allows for a stateful
//! sequence.
//!
//! Limitations:
//!
//! - one fuel wallet and one L1.
//! - one signer (`add_vkey`)
//! - tags are unique (for the key)
//! - a commit for a lock is a commit on the triple `(lock, tag, index)`.
//!   Retries on a lock are possible, but only on the same tag and index,
//!   and only once the previous attempt is declared `Ko`.
//! - limited delegation, but covers enough to set a delegation and
//!   change the delegation credential.

use std::collections::BTreeMap;

use konduit_data::{Lock, Tag};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::{l1, l2};

mod keys;
use keys::Keys;

mod commitments;
use commitments::Commitments;

#[derive(Debug, Clone, Default, Serialize, Deserialize, Encode, Decode)]
pub struct Config {
    #[n(0)]
    keys: Keys,
    #[n(1)]
    l1: l1::Config,
    #[n(2)]
    l2s: BTreeMap<Tag, l2::Config>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Encode, Decode)]
pub struct State {
    #[n(0)]
    keys: Keys,
    #[n(1)]
    l1: l1::State,
    #[n(2)]
    l2s: BTreeMap<Tag, l2::State>,
    #[n(3)]
    commitments: Commitments,
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    // -- Config passthroughs --
    pub fn wallet_key(&self) -> Option<[u8; 32]> {
        self.keys.wallet_key()
    }
    pub fn set_wallet_key(&mut self, key: Option<[u8; 32]>) {
        self.keys.set_wallet_key(key);
    }
    pub fn add_vkey(&self) -> Option<[u8; 32]> {
        self.keys.add_vkey()
    }
    pub fn set_add_vkey(&mut self, key: Option<[u8; 32]>) {
        self.keys.set_add_vkey(key);
    }

    // -- Global commitments passthroughs --
    pub fn get_tag(&self, lock: &Lock) -> Option<Tag> {
        self.commitments.get(lock)
    }

    pub fn set_tag(&mut self, lock: Lock, tag: Tag) -> Result<(), commitments::Error> {
        self.commitments.try_set(lock, tag)?;
        Ok(())
    }

    // -- L1 --
    pub fn l1(&self) -> &l1::State {
        &self.l1
    }
    pub fn l1_mut(&mut self) -> &mut l1::State {
        &mut self.l1
    }

    // -- L2s, keyed by tag --
    pub fn l2(&self, tag: &Tag) -> Option<&l2::State> {
        self.l2s.get(tag)
    }
    pub fn l2_mut(&mut self, tag: &Tag) -> Option<&mut l2::State> {
        self.l2s.get_mut(tag)
    }
    pub fn insert_l2(&mut self, tag: Tag, state: l2::State) {
        self.l2s.insert(tag, state);
    }
    pub fn remove_l2(&mut self, tag: &Tag) -> Option<l2::State> {
        self.l2s.remove(tag)
    }
    pub fn tags(&self) -> impl Iterator<Item = &Tag> {
        self.l2s.keys()
    }
}
