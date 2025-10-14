use anyhow::{Result, anyhow};

use pallas_primitives::PlutusData;

use super::base::{Amount, Index};
use super::plutus::{self, PData};

#[derive(Debug, Clone)]
pub struct Exclude(pub Vec<Index>);

impl PData for Exclude {
    fn to_plutus_data(self: &Self) -> PlutusData {
        plutus::list(&self.0.iter().map(|x| x.to_plutus_data()).collect())
    }

    fn from_plutus_data(d: &PlutusData) -> Result<Exclude> {
        let v = plutus::unlist(d)?;
        let x: Vec<Index> = v
            .iter()
            .map(|x| PData::from_plutus_data(x))
            .collect::<Result<Vec<Index>>>()?;
        Ok(Exclude(x))
    }
}

#[derive(Debug, Clone)]
pub struct Squash {
    amount: Amount,
    index: Index,
    exclude: Exclude,
}

impl Squash {
    pub fn new(amount: Amount, index: Index, exclude: Exclude) -> Self {
        Self {
            amount,
            index,
            exclude,
        }
    }
}

impl PData for Squash {
    fn to_plutus_data(self: &Self) -> PlutusData {
        let data = plutus::list(&vec![
            self.amount.to_plutus_data(),
            self.index.to_plutus_data(),
            self.exclude.to_plutus_data(),
        ]);
        data
    }

    fn from_plutus_data(d: &PlutusData) -> Result<Self> {
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
