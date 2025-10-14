use anyhow::{Result, anyhow};
use pallas_primitives::PlutusData;

use super::plutus::{self, PData, constr, constr_arr, unlist};
use super::step::Steps;

#[derive(Debug, Clone)]
pub enum Redeemer {
    Batch,
    Main(Steps),
    Mutual,
}

impl Redeemer {
    pub fn new_batch() -> Self {
        Self::Batch
    }

    pub fn new_main(steps: Steps) -> Self {
        Self::Main(steps)
    }

    pub fn new_mutual() -> Self {
        Self::Mutual
    }
}

impl PData for Redeemer {
    fn to_plutus_data(self: &Self) -> PlutusData {
        match &self {
            Redeemer::Batch => constr(0, vec![]),
            Redeemer::Main(steps) => constr(1, steps.0.iter().map(PData::to_plutus_data).collect()),
            Redeemer::Mutual => constr(2, vec![]),
        }
    }

    fn from_plutus_data(d: &PlutusData) -> Result<Redeemer> {
        let (constr_index, v) = &plutus::unconstr(d)?;
        match constr_index {
            0 => match &v[..] {
                [] => Ok(Self::new_batch()),
                _ => Err(anyhow!("bad length")),
            },
            1 => match &v[..] {
                [a] => Ok(Self::new_main(PData::from_plutus_data(&a)?)),
                _ => Err(anyhow!("bad length")),
            },
            2 => match &v[..] {
                [] => Ok(Self::new_mutual()),
                _ => Err(anyhow!("bad length")),
            },
            _ => Err(anyhow!("Bad constr tag")),
        }
    }
}
