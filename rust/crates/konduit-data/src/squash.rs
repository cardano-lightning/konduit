use anyhow::anyhow;
use cardano_tx_builder::{PlutusData, Signature, SigningKey, VerificationKey};

use crate::{
    SquashBody,
    utils::{signature_from_plutus_data, signature_to_plutus_data},
};

#[derive(Debug, Clone)]
pub struct Squash {
    pub squash_body: SquashBody,
    pub signature: Signature,
}

impl Squash {
    pub fn new(squash_body: SquashBody, signature: Signature) -> Self {
        Self {
            squash_body,
            signature,
        }
    }

    pub fn amount(&self) -> u64 {
        self.squash_body.amount
    }

    pub fn make(signing_key: SigningKey, tag: Vec<u8>, squash_body: SquashBody) -> Self {
        let signature = signing_key.sign(squash_body.tagged_bytes(tag));
        Self::new(squash_body, signature)
    }

    pub fn verify(&self, verification_key: VerificationKey, tag: Vec<u8>) -> bool {
        verification_key.verify(self.squash_body.tagged_bytes(tag), &self.signature)
    }
}

impl<'a> TryFrom<Vec<PlutusData<'a>>> for Squash {
    type Error = anyhow::Error;

    fn try_from(value: Vec<PlutusData<'a>>) -> anyhow::Result<Self> {
        Self::try_from(<[PlutusData; 2]>::try_from(value).map_err(|_| anyhow!("Bad length"))?)
    }
}

impl<'a> TryFrom<[PlutusData<'a>; 2]> for Squash {
    type Error = anyhow::Error;

    fn try_from(value: [PlutusData<'a>; 2]) -> anyhow::Result<Self> {
        let [a, b] = value;
        Ok(Self::new(
            SquashBody::try_from(a)?,
            signature_from_plutus_data(&b)?,
        ))
    }
}

impl<'a> TryFrom<&PlutusData<'a>> for Squash {
    type Error = anyhow::Error;

    fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
        Self::try_from(<[PlutusData; 2]>::try_from(data)?)
    }
}

impl<'a> From<Squash> for [PlutusData<'a>; 2] {
    fn from(value: Squash) -> Self {
        [
            PlutusData::from(&value.squash_body),
            signature_to_plutus_data(value.signature),
        ]
    }
}

impl<'a> From<Squash> for PlutusData<'a> {
    fn from(value: Squash) -> Self {
        Self::list(<[PlutusData; 2]>::from(value).to_vec())
    }
}
