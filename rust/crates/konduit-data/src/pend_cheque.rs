use anyhow::{Result, anyhow};

use pallas_primitives::PlutusData;

use super::base::{Amount, Hash32, Timestamp};
use super::plutus::{self, PData};

#[derive(Debug, Clone)]
pub struct PendCheque {
    amount: Amount,
    timeout: Timestamp,
    image: Hash32,
}

impl PendCheque {
    pub fn new(amount: Amount, timeout: Timestamp, image: Hash32) -> Self {
        Self {
            amount,
            timeout,
            image,
        }
    }
}

impl PData for PendCheque {
    fn to_plutus_data(self: &Self) -> PlutusData {
        let data = plutus::list(&vec![
            self.amount.to_plutus_data(),
            self.timeout.to_plutus_data(),
            self.image.to_plutus_data(),
        ]);
        data
    }

    fn from_plutus_data(d: &PlutusData) -> Result<PendCheque> {
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
pub struct PendCheques(pub Vec<PendCheque>);

impl PData for PendCheques {
    fn to_plutus_data(self: &Self) -> PlutusData {
        plutus::list(&self.0.iter().map(|x| x.to_plutus_data()).collect())
    }

    fn from_plutus_data(d: &PlutusData) -> Result<Self> {
        let v = plutus::unlist(d)?;
        let x: Vec<PendCheque> = v
            .iter()
            .map(|x| PendCheque::from_plutus_data(x))
            .collect::<Result<Vec<PendCheque>>>()?;
        Ok(PendCheques(x))
    }
}
