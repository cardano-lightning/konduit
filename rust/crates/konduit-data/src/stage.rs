use anyhow::{Result, anyhow};
use pallas_primitives::PlutusData;

use super::plutus::{self, PData, constr};

use super::{
    base::{Amount, Timestamp},
    pend_cheque::PendCheques,
};

#[derive(Debug, Clone)]
pub enum Stage {
    Opened(Amount),
    Closed(Amount, Timestamp),
    Responded(Amount, PendCheques),
}

impl Stage {
    pub fn new_opened(amount: Amount) -> Self {
        Self::Opened(amount)
    }

    pub fn new_closed(amount: Amount, timeout: Timestamp) -> Self {
        Self::Closed(amount, timeout)
    }

    pub fn new_responded(amount: Amount, pends: PendCheques) -> Self {
        Self::Responded(amount, pends)
    }
}

impl PData for Stage {
    fn to_plutus_data(self: &Self) -> PlutusData {
        match &self {
            Stage::Opened(amount) => constr(0, vec![amount.to_plutus_data()]),
            Stage::Closed(amount, timeout) => {
                constr(1, vec![amount.to_plutus_data(), timeout.to_plutus_data()])
            }
            Stage::Responded(amount, pend_cheques) => constr(
                2,
                vec![amount.to_plutus_data(), pend_cheques.to_plutus_data()],
            ),
        }
    }

    fn from_plutus_data(d: &PlutusData) -> Result<Stage> {
        let (constr_index, v) = &plutus::unconstr(d)?;
        match constr_index {
            0 => match &v[..] {
                [a] => Ok(Self::new_opened(PData::from_plutus_data(&a)?)),
                _ => Err(anyhow!("bad length")),
            },
            1 => match &v[..] {
                [a, b] => Ok(Self::new_closed(
                    PData::from_plutus_data(&a)?,
                    PData::from_plutus_data(&b)?,
                )),
                _ => Err(anyhow!("bad length")),
            },
            2 => match &v[..] {
                [a, b] => Ok(Self::new_responded(
                    PData::from_plutus_data(&a)?,
                    PData::from_plutus_data(&b)?,
                )),
                _ => Err(anyhow!("bad length")),
            },
            _ => Err(anyhow!("Bad constr tag")),
        }
    }
}
