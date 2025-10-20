use anyhow::{Error, Result, anyhow};
use cardano_tx_builder::{PlutusData, Signature, VerificationKey};

use crate::{
    cheque::Cheque, cheque_body::ChequeBody, signature_from_plutus_data, signature_to_plutus_data,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Unlocked {
    pub cheque_body: ChequeBody,
    pub signature: Signature,
    pub secret: [u8; 32],
}

impl Unlocked {
    pub fn new(
        cheque_body: ChequeBody,
        signature: Signature,
        secret: [u8; 32],
    ) -> anyhow::Result<Self> {
        if !cheque_body.is_secret(&secret) {
            Err(anyhow!("Bad secret"))?;
        }
        Ok(Self::new_no_verify(cheque_body, signature, secret))
    }

    pub fn new_no_verify(cheque_body: ChequeBody, signature: Signature, secret: [u8; 32]) -> Self {
        Self {
            cheque_body,
            signature,
            secret,
        }
    }

    pub fn cheque(&self) -> Cheque {
        Cheque::new(self.cheque_body.clone(), self.signature.clone())
    }

    pub fn verify(&self, timeout: u64, verification_key: VerificationKey, tag: Vec<u8>) -> bool {
        // Assume secret is verified in constructor
        self.cheque().verify(&verification_key, tag) && self.cheque_body.timeout < timeout
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
        Ok(Self::new(
            ChequeBody::try_from(a)?,
            signature_from_plutus_data(&b)?,
            <&[u8; 32]>::try_from(&c)?.clone(),
        )?)
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
