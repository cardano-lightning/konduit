use crate::{cheque::Cheque, unlocked::Unlocked};
use anyhow::{Error, Result, anyhow};
use cardano_tx_builder::PlutusData;

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
        let (variant, fields): (u64, Vec<PlutusData<'_>>) = (&data).try_into()?;

        return match variant {
            _ if variant == 0 => {
                try_unlocked(fields).map_err(|e| e.context("invalid 'Unlocked' variant"))
            }
            _ if variant == 1 => {
                try_cheque(fields).map_err(|e| e.context("invalid 'Cheque' variant"))
            }
            _ => Err(anyhow!("unknown variant: {variant}")),
        };

        fn try_unlocked(fields: Vec<PlutusData<'_>>) -> anyhow::Result<MixedCheque> {
            Ok(MixedCheque::new_unlocked(Unlocked::try_from(fields)?))
        }

        fn try_cheque(fields: Vec<PlutusData<'_>>) -> anyhow::Result<MixedCheque> {
            Ok(MixedCheque::new_cheque(Cheque::try_from(fields)?))
        }
    }
}

impl<'a> From<MixedCheque> for PlutusData<'a> {
    fn from(value: MixedCheque) -> Self {
        match value {
            MixedCheque::Unlocked(unlocked) => PlutusData::constr(0, Vec::from(unlocked)),
            MixedCheque::Cheque(cheque) => PlutusData::constr(1, Vec::from(cheque)),
        }
    }
}
