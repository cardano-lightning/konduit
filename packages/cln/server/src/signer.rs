use konduit_data::{
    ChequeBody, Locked, Signature, SigningKey, Squash, SquashBody, Tag, Verified, VerifyingKey,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(with = "hex::serde")]
    pub key: [u8; 32],
}

pub struct Signer {
    key: SigningKey,
}

impl Signer {
    pub fn new(config: Config) -> Self {
        Self {
            key: SigningKey::from(config.key),
        }
    }

    pub fn verifying_key(&self) -> VerifyingKey {
        self.key.verifying_key()
    }

    pub fn verify(&self, msg: &[u8], sig: &Signature) -> bool {
        self.verifying_key().verify(msg, sig)
    }

    pub fn locked(&self, tag: Tag, body: ChequeBody) -> Locked<Verified> {
        Locked::make(&self.key, &tag, body)
    }

    pub fn squash(&self, tag: Tag, body: SquashBody) -> Squash<Verified> {
        Squash::make(&self.key, &tag, body)
    }
}
