use anyhow::{Error, Result, anyhow};
use cardano_tx_builder::PlutusData;

use crate::{base::Signature, cheque_body::ChequeBody};

#[derive(Debug, Clone)]
pub struct Cheque {
    pub cheque_body: ChequeBody,
    pub signature: Signature,
}

impl Cheque {
    pub fn new(cheque_body: ChequeBody, signature: Signature) -> Self {
        Self {
            cheque_body,
            signature,
        }
    }
}

impl<'a> TryFrom<Vec<PlutusData<'a>>> for Cheque {
    type Error = Error;

    fn try_from(value: Vec<PlutusData<'a>>) -> Result<Self> {
        Self::try_from(<[PlutusData; 2]>::try_from(value).map_err(|_| anyhow!("Bad length"))?)
    }
}

impl<'a> TryFrom<[PlutusData<'a>; 2]> for Cheque {
    type Error = Error;

    fn try_from(value: [PlutusData<'a>; 2]) -> Result<Self> {
        let [a, b] = value;
        Ok(Self::new(ChequeBody::try_from(a)?, Signature::try_from(b)?))
    }
}

impl<'a> TryFrom<&PlutusData<'a>> for Cheque {
    type Error = Error;

    fn try_from(data: &PlutusData<'a>) -> Result<Self> {
        Self::try_from(<[PlutusData; 2]>::try_from(data)?)
    }
}

impl<'a> From<Cheque> for [PlutusData<'a>; 2] {
    fn from(value: Cheque) -> Self {
        [
            PlutusData::from(value.cheque_body),
            PlutusData::from(value.signature),
        ]
    }
}

impl<'a> From<Cheque> for PlutusData<'a> {
    fn from(value: Cheque) -> Self {
        Self::list(<[PlutusData; 2]>::from(value).to_vec())
    }
}
