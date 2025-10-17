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
        Ok(Self::new(ChequeBody::try_from(a)?, Signature::try_from(b)?))
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
            PlutusData::from(cheque.signature),
        ]
    }
}
