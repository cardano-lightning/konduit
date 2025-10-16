use anyhow::{Error, Result, anyhow};
use cardano_tx_builder::PlutusData;

use crate::{cheque::Cheque, unlocked::Unlocked};

#[derive(Debug, Clone)]
pub enum MixedCheque {
    Unlocked(Unlocked),
    Cheque(Cheque),
}

impl MixedCheque {
    pub fn new_unlocked(unlocked: Unlocked) -> Self {
        Self::Unlocked(unlocked)
    }

    pub fn new_cheque(cheque: Cheque) -> Self {
        Self::Cheque(cheque)
    }
}

impl<'a> TryFrom<PlutusData<'a>> for MixedCheque {
    type Error = Error;

    fn try_from(data: PlutusData<'a>) -> Result<Self> {
        let (xi, fields) = data.as_constr().ok_or(anyhow!("Expect constr"))?;
        let fields = fields.collect::<Vec<PlutusData>>();
        if xi == 0 {
            Ok(Self::new_unlocked(Unlocked::try_from(fields)?))
        } else if xi == 1 {
            Ok(Self::new_cheque(Cheque::try_from(fields)?))
        } else {
            Err(anyhow!("Bad tag"))
        }
    }
}

impl<'a> From<MixedCheque> for PlutusData<'a> {
    fn from(value: MixedCheque) -> Self {
        match value {
            MixedCheque::Unlocked(x) => PlutusData::constr(0, <[PlutusData; 3]>::from(x).to_vec()),
            MixedCheque::Cheque(x) => PlutusData::constr(1, <[PlutusData; 2]>::from(x).to_vec()),
        }
    }
}
