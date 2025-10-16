use anyhow::{Error, Result, anyhow};
use cardano_tx_builder::PlutusData;

use crate::steps::Steps;

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

impl<'a> TryFrom<PlutusData<'a>> for Redeemer {
    type Error = Error;

    fn try_from(data: PlutusData<'a>) -> Result<Self> {
        let (xi, fields) = data.as_constr().ok_or(anyhow!("Expect constr"))?;
        let fields = fields.collect::<Vec<PlutusData>>();
        if xi == 0 {
            let [] = <[PlutusData; 0]>::try_from(fields).map_err(|_| anyhow!("Bad length"))?;
            Ok(Self::new_batch())
        } else if xi == 1 {
            let [a] = <[PlutusData; 1]>::try_from(fields).map_err(|_| anyhow!("Bad length"))?;
            Ok(Self::new_main(Steps::try_from(&a)?))
        } else if xi == 2 {
            let [] = <[PlutusData; 0]>::try_from(fields).map_err(|_| anyhow!("Bad length"))?;
            Ok(Self::new_mutual())
        } else {
            Err(anyhow!("Bad tag"))
        }
    }
}

impl<'a> From<Redeemer> for PlutusData<'a> {
    fn from(value: Redeemer) -> Self {
        match value {
            Redeemer::Batch => PlutusData::constr(0, vec![]),
            Redeemer::Main(steps) => PlutusData::constr(0, vec![PlutusData::from(steps)]),
            Redeemer::Mutual => PlutusData::constr(0, vec![]),
        }
    }
}
