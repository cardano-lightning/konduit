use anyhow::{Error, Result, anyhow};
use cardano_tx_builder::{PlutusData, Signature, SigningKey, VerificationKey};

use crate::{cheque_body::ChequeBody, signature_from_plutus_data, signature_to_plutus_data};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
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

    pub fn make(signing_key: SigningKey, tag: Vec<u8>, cheque_body: ChequeBody) -> Self {
        let signature = signing_key.sign(cheque_body.tagged_bytes(tag));
        Self::new(cheque_body, signature)
    }

    pub fn verify(&self, verification_key: &VerificationKey, tag: Vec<u8>) -> bool {
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
