use anyhow::anyhow;
use cardano_tx_builder::{PlutusData, constr};

use crate::{
    base::{Amount, Timestamp},
    pendings::Pendings,
};

#[derive(Debug, Clone)]
pub enum Stage {
    Opened(Amount),
    Closed(Amount, Timestamp),
    Responded(Amount, Pendings),
}

impl Stage {
    pub fn new_opened(amount: Amount) -> Self {
        Self::Opened(amount)
    }

    pub fn new_closed(amount: Amount, timeout: Timestamp) -> Self {
        Self::Closed(amount, timeout)
    }

    pub fn new_responded(amount: Amount, pendings: Pendings) -> Self {
        Self::Responded(amount, pendings)
    }
}

impl<'a> TryFrom<PlutusData<'a>> for Stage {
    type Error = anyhow::Error;

    fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
        let (variant, fields): (u64, Vec<PlutusData<'_>>) = (&data).try_into()?;

        return match variant {
            _ if variant == 0 => {
                try_opened(fields).map_err(|e| e.context("invalid 'Opened' variant"))
            }
            _ if variant == 1 => {
                try_closed(fields).map_err(|e| e.context("invalid 'Closed' variant"))
            }
            _ if variant == 2 => {
                try_responded(fields).map_err(|e| e.context("invalid 'Responded' variant"))
            }
            _ => Err(anyhow!("unknown variant: {variant}")),
        };

        fn try_opened(fields: Vec<PlutusData<'_>>) -> anyhow::Result<Stage> {
            let [a] = <[PlutusData; 1]>::try_from(fields)
                .map_err(|vec| anyhow!("expected 1 field, found {}", vec.len()))?;

            Ok(Stage::new_opened(Amount::try_from(a)?))
        }

        fn try_closed(fields: Vec<PlutusData<'_>>) -> anyhow::Result<Stage> {
            let [a, b] = <[PlutusData; 2]>::try_from(fields)
                .map_err(|vec| anyhow!("expected 2 fields, found {}", vec.len()))?;

            Ok(Stage::new_closed(
                Amount::try_from(a)?,
                Timestamp::try_from(b)?,
            ))
        }

        fn try_responded(fields: Vec<PlutusData<'_>>) -> anyhow::Result<Stage> {
            let [a, b] = <[PlutusData; 2]>::try_from(fields)
                .map_err(|vec| anyhow!("expected 2 fields, found {}", vec.len()))?;

            Ok(Stage::new_responded(
                Amount::try_from(a)?,
                Pendings::try_from(&b)?,
            ))
        }
    }
}

impl<'a> From<Stage> for PlutusData<'a> {
    fn from(value: Stage) -> Self {
        match value {
            Stage::Opened(a) => constr!(0, a),
            Stage::Closed(a, b) => constr!(1, a, b),
            Stage::Responded(a, b) => constr!(2, a, b),
        }
    }
}
