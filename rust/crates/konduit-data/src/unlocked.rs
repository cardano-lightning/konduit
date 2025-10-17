use anyhow::{Error, Result, anyhow};
use cardano_tx_builder::PlutusData;

use crate::{
    base::{Secret, Signature},
    cheque_body::ChequeBody,
};

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
            Signature::try_from(b)?,
            Secret::try_from(c)?,
        ))
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
            PlutusData::from(unlocked.signature),
            PlutusData::from(unlocked.secret),
        ]
    }
}
