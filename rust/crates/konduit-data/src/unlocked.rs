use anyhow::{Error, Result, anyhow};
use cardano_tx_builder::PlutusData;

use crate::base::{Secret, Signature};
use crate::cheque_body::ChequeBody;

#[derive(Debug, Clone)]
pub struct Unlocked {
    pub cheque_body: ChequeBody,
    pub signature: Signature,
    pub secret: Secret,
}

impl Unlocked {
    pub fn new(cheque_body: ChequeBody, signature: Signature, secret: Secret) -> Self {
        Self {
            cheque_body,
            signature,
            secret,
        }
    }
}

impl<'a> TryFrom<Vec<PlutusData<'a>>> for Unlocked {
    type Error = Error;

    fn try_from(value: Vec<PlutusData<'a>>) -> Result<Self> {
        Self::try_from(<[PlutusData; 3]>::try_from(value).map_err(|_| anyhow!("Bad length"))?)
    }
}

impl<'a> TryFrom<[PlutusData<'a>; 3]> for Unlocked {
    type Error = Error;

    fn try_from(value: [PlutusData<'a>; 3]) -> Result<Self> {
        let [a, b, c] = value;
        Ok(Self::new(
            ChequeBody::try_from(a)?,
            Signature::try_from(b)?,
            Secret::try_from(c)?,
        ))
    }
}

impl<'a> TryFrom<&PlutusData<'a>> for Unlocked {
    type Error = Error;

    fn try_from(data: &PlutusData<'a>) -> Result<Self> {
        Self::try_from(<[PlutusData; 3]>::try_from(data)?)
    }
}

impl<'a> From<Unlocked> for [PlutusData<'a>; 3] {
    fn from(value: Unlocked) -> Self {
        [
            PlutusData::from(value.cheque_body),
            PlutusData::from(value.signature),
            PlutusData::from(value.secret),
        ]
    }
}

impl<'a> From<Unlocked> for PlutusData<'a> {
    fn from(value: Unlocked) -> Self {
        Self::list(<[PlutusData; 3]>::from(value).to_vec())
    }
}
