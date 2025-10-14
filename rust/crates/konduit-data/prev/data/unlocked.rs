use anyhow::{Result, anyhow};
use pallas_primitives::PlutusData;

use super::base::{Secret, Signature};
use super::cheque_body::ChequeBody;
use super::plutus::{self, PData};

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

impl PData for Unlocked {
    fn to_plutus_data(self: &Self) -> PlutusData {
        let data = plutus::list(&vec![
            self.cheque_body.to_plutus_data(),
            self.signature.to_plutus_data(),
            self.secret.to_plutus_data(),
        ]);
        data
    }

    fn from_plutus_data(d: &PlutusData) -> Result<Unlocked> {
        match &plutus::unlist(d)?[..] {
            [a, b, c] => {
                let r = Self::new(
                    PData::from_plutus_data(a)?,
                    PData::from_plutus_data(b)?,
                    PData::from_plutus_data(c)?,
                );
                Ok(r)
            }
            _ => Err(anyhow!("bad length")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Unlockeds(pub Vec<Unlocked>);

impl PData for Unlockeds {
    fn to_plutus_data(self: &Self) -> PlutusData {
        plutus::list(&self.0.iter().map(|x| x.to_plutus_data()).collect())
    }

    fn from_plutus_data(d: &PlutusData) -> Result<Self> {
        let v = plutus::unlist(d)?;
        let x: Vec<Unlocked> = v
            .iter()
            .map(|x| PData::from_plutus_data(x))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self(x))
    }
}
