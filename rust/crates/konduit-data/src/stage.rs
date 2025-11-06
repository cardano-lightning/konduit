use anyhow::anyhow;
use cardano_tx_builder::{PlutusData, constr};
use serde::{Deserialize, Serialize};

use crate::{Duration, Pending};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Stage {
    Opened(u64),
    Closed(u64, Duration),
    Responded(u64, Vec<Pending>),
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

            Ok(Stage::Opened(u64::try_from(&a)?))
        }

        fn try_closed(fields: Vec<PlutusData<'_>>) -> anyhow::Result<Stage> {
            let [a, b] = <[PlutusData; 2]>::try_from(fields)
                .map_err(|vec| anyhow!("expected 2 field, found {}", vec.len()))?;

            Ok(Stage::Closed(u64::try_from(&a)?, Duration::try_from(&b)?))
        }

        fn try_responded(fields: Vec<PlutusData<'_>>) -> anyhow::Result<Stage> {
            let [a, b] = <[PlutusData; 2]>::try_from(fields)
                .map_err(|vec| anyhow!("expected 2 field, found {}", vec.len()))?;
            let pendings: Vec<Pending> = b
                .as_list()
                .ok_or(anyhow!("Expected list"))?
                .into_iter()
                .map(|x| Pending::try_from(&x))
                .collect::<anyhow::Result<Vec<Pending>>>()?;
            Ok(Stage::Responded(u64::try_from(&a)?, pendings))
        }
    }
}

impl<'a> From<Stage> for PlutusData<'a> {
    fn from(value: Stage) -> Self {
        match value {
            Stage::Opened(a) => constr!(0, a),
            Stage::Closed(a, b) => constr!(1, a, b),
            Stage::Responded(a, b) => constr!(2, a, PlutusData::list(b)),
        }
    }
}
