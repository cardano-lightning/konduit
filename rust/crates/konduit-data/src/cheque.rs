use anyhow::{Error, Result, anyhow};
use cardano_tx_builder::{PlutusData, Signature, SigningKey, VerificationKey};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::cheque_body::ChequeBody;
use crate::utils::{signature_from_plutus_data, signature_to_plutus_data};
use crate::{Tag, plutus_data_serde};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Cheque {
    pub cheque_body: ChequeBody,
    pub signature: Signature,
}

impl Serialize for Cheque {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        plutus_data_serde::serialize(self, serializer)
    }
}

impl<'de> Deserialize<'de> for Cheque {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        plutus_data_serde::deserialize::<D, Self>(deserializer)
    }
}

impl Cheque {
    pub fn new(cheque_body: ChequeBody, signature: Signature) -> Self {
        Self {
            cheque_body,
            signature,
        }
    }

    pub fn make(signing_key: &SigningKey, tag: &Tag, cheque_body: ChequeBody) -> Self {
        let signature = signing_key.sign(cheque_body.tagged_bytes(tag));
        Self::new(cheque_body, signature)
    }

    pub fn verify(&self, verification_key: &VerificationKey, tag: &Tag) -> bool {
        verification_key.verify(self.cheque_body.tagged_bytes(tag), &self.signature)
    }
}

impl<'a> TryFrom<&PlutusData<'a>> for Cheque {
    type Error = Error;

    fn try_from(data: &PlutusData<'a>) -> Result<Self> {
        let fields: Vec<PlutusData<'_>> = Vec::try_from(data)?;
        Cheque::try_from(fields)
    }
}

impl<'a> TryFrom<Vec<PlutusData<'a>>> for Cheque {
    type Error = Error;

    fn try_from(list: Vec<PlutusData<'a>>) -> Result<Self> {
        let [a, b] = <[PlutusData; 2]>::try_from(list).map_err(|_| anyhow!("invalid 'Cheque'"))?;
        Ok(Self::new(
            ChequeBody::try_from(a)?,
            signature_from_plutus_data(&b)?,
        ))
    }
}

impl<'a> From<Cheque> for PlutusData<'a> {
    fn from(cheque: Cheque) -> Self {
        PlutusData::list(Vec::from(cheque))
    }
}

impl<'a> From<Cheque> for Vec<PlutusData<'a>> {
    fn from(cheque: Cheque) -> Self {
        vec![
            PlutusData::from(cheque.cheque_body),
            signature_to_plutus_data(cheque.signature),
        ]
    }
}
