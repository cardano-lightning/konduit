use crate::{
    utils::{signature_from_plutus_data, signature_to_plutus_data},
    SquashBody, Tag,
};
use anyhow::anyhow;
use cardano_sdk::{PlutusData, Signature, SigningKey, VerificationKey};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Squash {
    pub body: SquashBody,
    #[serde_as(as = "serde_with::hex::Hex")]
    pub signature: Signature,
}

impl Squash {
    pub fn new(body: SquashBody, signature: Signature) -> Self {
        Self { body, signature }
    }

    pub fn amount(&self) -> u64 {
        self.body.amount
    }

    pub fn index(&self) -> u64 {
        self.body.index
    }

    pub fn is_index_squashed(&self, index: u64) -> bool {
        self.body.is_index_squashed(index)
    }

    pub fn make(signing_key: &SigningKey, tag: &Tag, body: SquashBody) -> Self {
        let signature = signing_key.sign(body.tagged_bytes(tag));
        Self::new(body, signature)
    }

    pub fn verify(&self, verification_key: &VerificationKey, tag: &Tag) -> bool {
        verification_key.verify(self.body.tagged_bytes(tag), &self.signature)
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

impl<'a> TryFrom<PlutusData<'a>> for Squash {
    type Error = anyhow::Error;

    fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
        Self::try_from(<[PlutusData; 2]>::try_from(&data)?)
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
            PlutusData::from(&value.body),
            signature_to_plutus_data(value.signature),
        ]
    }
}

impl<'a> From<Squash> for PlutusData<'a> {
    fn from(value: Squash) -> Self {
        Self::list(<[PlutusData; 2]>::from(value).to_vec())
    }
}

#[cfg(test)]
mod tests {
    use crate::Indexes;

    use super::*;

    #[test]
    fn test_squash_round_trip() {
        let sk = SigningKey::from([0; 32]);
        let tag = Tag::from([1; 20].to_vec());
        let body = SquashBody::new_no_verify(120309, 123, Indexes::new([22].to_vec()).unwrap());
        let original = Squash::make(&sk, &tag, body);

        println!("{}", serde_json::to_string_pretty(&original).unwrap());
        let ser = serde_json::to_vec(&original).expect("Failed to serialize ChequeBody");

        let de: Squash = serde_json::from_slice(&ser).expect("Failed to deserialize ChequeBody");

        assert_eq!(original.body, de.body);
    }
}
