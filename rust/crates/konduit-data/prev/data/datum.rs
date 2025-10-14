use super::base::Hash28;
use super::constants::Constants;
use super::stage::Stage;
use anyhow::anyhow;

use pallas_primitives::PlutusData;

use super::plutus::{self, PData};

pub struct Datum {
    pub own_hash: Hash28,
    pub constants: Constants,
    pub stage: Stage,
}

impl Datum {
    pub fn new(own_hash: Hash28, constants: Constants, stage: Stage) -> Self {
        Self {
            own_hash,
            constants,
            stage,
        }
    }
}

impl PData for Datum {
    fn to_plutus_data(self: &Self) -> PlutusData {
        let data = plutus::list(&vec![
            self.own_hash.to_plutus_data(),
            self.constants.to_plutus_data(),
            self.stage.to_plutus_data(),
        ]);
        data
    }

    fn from_plutus_data(d: &PlutusData) -> anyhow::Result<Datum> {
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
