use anyhow::{Error, Result};
use cardano_tx_builder::PlutusData;

use crate::base::ScriptHash;
use crate::constants::Constants;
use crate::stage::Stage;

#[derive(Debug, Clone)]
pub struct Datum {
    pub own_hash: ScriptHash,
    pub constants: Constants,
    pub stage: Stage,
}

impl Datum {
    pub fn new(own_hash: ScriptHash, constants: Constants, stage: Stage) -> Self {
        Self {
            own_hash,
            constants,
            stage,
        }
    }
}

impl<'a> TryFrom<PlutusData<'a>> for Datum {
    type Error = Error;

    fn try_from(data: PlutusData<'a>) -> Result<Self> {
        let [a, b, c] = <[PlutusData; 3]>::try_from(&data)?;
        Ok(Self::new(
            ScriptHash::try_from(a)?,
            Constants::try_from(&b)?,
            Stage::try_from(c)?,
        ))
    }
}

impl<'a> From<Datum> for PlutusData<'a> {
    fn from(value: Datum) -> Self {
        Self::list(vec![
            PlutusData::from(value.own_hash),
            PlutusData::from(value.constants),
            PlutusData::from(value.stage),
        ])
    }
}
