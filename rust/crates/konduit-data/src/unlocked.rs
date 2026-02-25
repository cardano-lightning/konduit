use crate::{
    Duration, Lock, Secret, Tag,
    cheque_body::ChequeBody,
    locked::Locked,
    utils::{signature_from_plutus_data, signature_to_plutus_data},
};
use anyhow::{Error, Result, anyhow};
use cardano_sdk::{PlutusData, Signature, VerificationKey};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Unlocked {
    pub body: ChequeBody,
    #[serde_as(as = "serde_with::hex::Hex")]
    pub signature: Signature,
    #[serde_as(as = "serde_with::hex::Hex")]
    pub secret: Secret,
}

impl Unlocked {
    pub fn new(locked: Locked, secret: Secret) -> anyhow::Result<Self> {
        if !locked.body.is_secret(&secret) {
            Err(anyhow!("Bad secret"))?;
        }
        Ok(Self::new_no_verify(locked.body, locked.signature, secret))
    }

    pub fn new_no_verify(body: ChequeBody, signature: Signature, secret: Secret) -> Self {
        Self {
            body,
            signature,
            secret,
        }
    }

    pub fn index(&self) -> u64 {
        self.body.index
    }

    pub fn amount(&self) -> u64 {
        self.body.amount
    }

    pub fn lock(&self) -> &Lock {
        &self.body.lock
    }

    pub fn locked(&self) -> Locked {
        Locked::new(self.body.clone(), self.signature)
    }

    pub fn verify(
        &self,
        timeout: &Duration,
        verification_key: &VerificationKey,
        tag: &Tag,
    ) -> bool {
        // Assume secret is verified in constructor
        self.locked().verify(verification_key, tag) && &self.body.timeout < timeout
    }

    pub fn verify_no_time(&self, verification_key: &VerificationKey, tag: &Tag) -> bool {
        // Assume secret is verified in constructor
        let locked = self.locked();
        locked.verify(verification_key, tag) && locked.body.is_secret(&self.secret)
    }
}

impl<'a> TryFrom<&PlutusData<'a>> for Unlocked {
    type Error = Error;

    fn try_from(data: &PlutusData<'a>) -> Result<Self> {
        let fields: Vec<PlutusData<'_>> = Vec::try_from(data)?;
        Unlocked::try_from(fields)
    }
}

impl<'a> TryFrom<Vec<PlutusData<'a>>> for Unlocked {
    type Error = Error;

    fn try_from(list: Vec<PlutusData<'a>>) -> Result<Self> {
        let [a, b, c] =
            <[PlutusData; 3]>::try_from(list).map_err(|_| anyhow!("invalid 'Unlocked'"))?;
        let locked = Locked::new(ChequeBody::try_from(a)?, signature_from_plutus_data(&b)?);
        Self::new(locked, Secret::try_from(&c)?)
    }
}

impl<'a> From<Unlocked> for PlutusData<'a> {
    fn from(unlocked: Unlocked) -> Self {
        PlutusData::list(Vec::from(unlocked))
    }
}

impl<'a> From<Unlocked> for Vec<PlutusData<'a>> {
    fn from(unlocked: Unlocked) -> Self {
        vec![
            PlutusData::from(unlocked.body),
            signature_to_plutus_data(unlocked.signature),
            PlutusData::from(unlocked.secret),
        ]
    }
}
