use anyhow::{Result, anyhow};
use pallas_primitives::PlutusData;

use super::{
    base::{Secret, Signature},
    cheque_body::ChequeBody,
    plutus::{self, PData, constr},
    unlocked::Unlocked,
};

#[derive(Debug, Clone)]
pub enum Mix {
    MUnlocked(Unlocked),
    MPend(MPend),
}

#[derive(Debug, Clone)]
pub struct MPend {
    cheque_body: ChequeBody,
    signature: Signature,
}

impl Mix {
    pub fn new_m_unlocked(cheque_body: ChequeBody, signature: Signature, secret: Secret) -> Self {
        Self::MUnlocked(Unlocked {
            cheque_body,
            signature,
            secret,
        })
    }

    pub fn new_m_pend(cheque_body: ChequeBody, signature: Signature) -> Self {
        Self::MPend(MPend {
            cheque_body,
            signature,
        })
    }
}

impl PData for Mix {
    fn to_plutus_data(self: &Self) -> PlutusData {
        match &self {
            Mix::MUnlocked(unlocked) => constr(
                0,
                vec![
                    unlocked.cheque_body.to_plutus_data(),
                    unlocked.signature.to_plutus_data(),
                    unlocked.secret.to_plutus_data(),
                ],
            ),
            Mix::MPend(m_pend) => constr(
                1,
                vec![
                    m_pend.cheque_body.to_plutus_data(),
                    m_pend.signature.to_plutus_data(),
                ],
            ),
        }
    }

    fn from_plutus_data(d: &PlutusData) -> Result<Self> {
        let (constr_index, v) = &plutus::unconstr(d)?;
        match constr_index {
            0 => match &v[..] {
                [a, b, c] => Ok(Self::new_m_unlocked(
                    PData::from_plutus_data(&a)?,
                    PData::from_plutus_data(&b)?,
                    PData::from_plutus_data(&c)?,
                )),
                _ => Err(anyhow!("bad length")),
            },
            1 => match &v[..] {
                [a, b] => Ok(Self::new_m_pend(
                    PData::from_plutus_data(&a)?,
                    PData::from_plutus_data(&b)?,
                )),
                _ => Err(anyhow!("bad length")),
            },
            _ => Err(anyhow!("Bad constr tag")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Mixs(Vec<Mix>);

impl PData for Mixs {
    fn to_plutus_data(self: &Self) -> PlutusData {
        plutus::list(&self.0.iter().map(|x| x.to_plutus_data()).collect())
    }

    fn from_plutus_data(d: &PlutusData) -> Result<Self> {
        let v = plutus::unlist(d)?;
        let x: Vec<Mix> = v
            .iter()
            .map(|x| PData::from_plutus_data(x))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self(x))
    }
}
