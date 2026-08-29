use konduit_data::{
    ChequeBody, Constants, Duration, Locked, SigningKey, Squash, SquashBody, Tag, Verified,
    VerifyingKey,
};
use konduit_tmp::Receipt;
use serde::{Deserialize, Serialize};

use crate::hash32;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub key: SigningKey,
    pub tag: Tag,
}

impl Config {
    pub fn tag(&self) -> &Tag {
        &self.tag
    }
    pub fn key(&self) -> &SigningKey {
        &self.key
    }
    pub fn verifying_key(&self) -> VerifyingKey {
        self.key.verifying_key()
    }
    pub fn new_squash(&self) -> Squash<Verified> {
        self.squash(SquashBody::zero())
    }
    pub fn squash(&self, body: SquashBody) -> Squash<Verified> {
        Squash::make(&self.key, &self.tag, body)
    }
    pub fn new_receipt(&self) -> Receipt {
        Receipt::new(self.new_squash())
    }
    pub fn locked_inner(&self, body: ChequeBody) -> Locked<Verified> {
        Locked::make(&self.key, &self.tag, body)
    }
    pub fn cardano_signing_key(&self) -> cardano_sdk::SigningKey {
        cardano_sdk::SigningKey::from(<[u8; 32]>::from(self.key.clone()))
    }

    pub fn constants(&self, adaptor_key: VerifyingKey, close_period: Duration) -> Constants {
        Constants {
            tag: self.tag.clone(),
            add_vkey: self.key.verifying_key(),
            sub_vkey: adaptor_key,
            close_period,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            key: hash32("account".as_bytes()).into(),
            tag: Tag::from(vec![]),
        }
    }
}

impl Config {
    pub fn new(seed: u8) -> Self {
        let key = hash32(&format!("account {}", seed).into_bytes()).into();
        Self {
            key,
            tag: Tag::generate((seed % 32) as usize),
        }
    }
}
