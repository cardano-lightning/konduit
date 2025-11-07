use anyhow::{Error, Result, anyhow};
use cardano_tx_builder::{PlutusData, Signature, VerificationKey};

use crate::{
    Duration, Secret, Tag,
    cheque::Cheque,
    cheque_body::ChequeBody,
    utils::{signature_from_plutus_data, signature_to_plutus_data},
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Unlocked {
    pub cheque_body: ChequeBody,
    pub signature: Signature,
    pub secret: Secret,
}

impl Unlocked {
    pub fn new(cheque: Cheque, secret: Secret) -> anyhow::Result<Self> {
        if !cheque.cheque_body.is_secret(&secret) {
            Err(anyhow!("Bad secret"))?;
        }
        Ok(Self::new_no_verify(
            cheque.cheque_body,
            cheque.signature,
            secret,
        ))
    }

    pub fn new_no_verify(cheque_body: ChequeBody, signature: Signature, secret: Secret) -> Self {
        Self {
            cheque_body,
            signature,
            secret,
        }
    }

    pub fn cheque(&self) -> Cheque {
        Cheque::new(self.cheque_body.clone(), self.signature)
    }

    pub fn verify(
        &self,
        timeout: &Duration,
        verification_key: &VerificationKey,
        tag: &Tag,
    ) -> bool {
        // Assume secret is verified in constructor
        self.cheque().verify(verification_key, tag) && &self.cheque_body.timeout < timeout
    }

    pub fn verify_no_time(&self, verification_key: &VerificationKey, tag: &Tag) -> bool {
        // Assume secret is verified in constructor
        let cheque = self.cheque();
        cheque.verify(verification_key, tag) && cheque.cheque_body.is_secret(&self.secret)
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
        let cheque = Cheque::new(ChequeBody::try_from(a)?, signature_from_plutus_data(&b)?);
        Self::new(cheque, Secret::try_from(&c)?)
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
            PlutusData::from(unlocked.cheque_body),
            signature_to_plutus_data(unlocked.signature),
            PlutusData::from(unlocked.secret),
        ]
    }
}
