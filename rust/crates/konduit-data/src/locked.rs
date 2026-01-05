use anyhow::{Error, Result, anyhow};
use cardano_tx_builder::{PlutusData, Signature, SigningKey, VerificationKey};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::cheque_body::ChequeBody;
use crate::utils::{signature_from_plutus_data, signature_to_plutus_data};
use crate::{Duration, Tag};

#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Locked {
    pub body: ChequeBody,
    #[serde_as(as = "serde_with::hex::Hex")]
    pub signature: Signature,
}

impl Locked {
    pub fn new(body: ChequeBody, signature: Signature) -> Self {
        Self { body, signature }
    }

    pub fn index(&self) -> u64 {
        self.body.index
    }

    pub fn amount(&self) -> u64 {
        self.body.amount
    }

    pub fn timeout(&self) -> Duration {
        self.body.timeout
    }

    pub fn make(signing_key: &SigningKey, tag: &Tag, body: ChequeBody) -> Self {
        let signature = signing_key.sign(body.tagged_bytes(tag));
        Self::new(body, signature)
    }

    pub fn verify(&self, verification_key: &VerificationKey, tag: &Tag) -> bool {
        verification_key.verify(self.body.tagged_bytes(tag), &self.signature)
    }
}

impl<'a> TryFrom<&PlutusData<'a>> for Locked {
    type Error = Error;

    fn try_from(data: &PlutusData<'a>) -> Result<Self> {
        let fields: Vec<PlutusData<'_>> = Vec::try_from(data)?;
        Locked::try_from(fields)
    }
}

impl<'a> TryFrom<Vec<PlutusData<'a>>> for Locked {
    type Error = Error;

    fn try_from(list: Vec<PlutusData<'a>>) -> Result<Self> {
        let [a, b] = <[PlutusData; 2]>::try_from(list).map_err(|_| anyhow!("invalid 'Locked'"))?;
        Ok(Self::new(
            ChequeBody::try_from(a)?,
            signature_from_plutus_data(&b)?,
        ))
    }
}

impl<'a> From<Locked> for PlutusData<'a> {
    fn from(locked: Locked) -> Self {
        PlutusData::list(Vec::from(locked))
    }
}

impl<'a> From<Locked> for Vec<PlutusData<'a>> {
    fn from(locked: Locked) -> Self {
        vec![
            PlutusData::from(locked.body),
            signature_to_plutus_data(locked.signature),
        ]
    }
}
