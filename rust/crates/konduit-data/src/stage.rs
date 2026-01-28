use anyhow::anyhow;
use cardano_tx_builder::{PlutusData, constr};
use serde::{Deserialize, Serialize};

use crate::{Duration, Pending, Used};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum Stage {
    Opened(u64, Vec<Used>),
    Closed(u64, Vec<Used>, Duration),
    Responded(u64, Vec<Pending>),
}

impl Stage {
    pub fn is_opened(&self) -> bool {
        if let Stage::Opened(_, _) = self {
            true
        } else {
            false
        }
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
            let [a, b] = <[PlutusData; 2]>::try_from(fields)
                .map_err(|vec| anyhow!("expected 1 field, found {}", vec.len()))?;
            let useds: Vec<Used> = b
                .as_list()
                .ok_or(anyhow!("Expected list"))?
                .map(|x| Used::try_from(&x))
                .collect::<anyhow::Result<Vec<Used>>>()?;

            Ok(Stage::Opened(u64::try_from(&a)?, useds))
        }

        fn try_closed(fields: Vec<PlutusData<'_>>) -> anyhow::Result<Stage> {
            let [a, b, c] = <[PlutusData; 3]>::try_from(fields)
                .map_err(|vec| anyhow!("expected 3 field, found {}", vec.len()))?;

            let useds: Vec<Used> = b
                .as_list()
                .ok_or(anyhow!("Expected list"))?
                .map(|x| Used::try_from(&x))
                .collect::<anyhow::Result<Vec<Used>>>()?;
            Ok(Stage::Closed(
                u64::try_from(&a)?,
                useds,
                Duration::try_from(&c)?,
            ))
        }

        fn try_responded(fields: Vec<PlutusData<'_>>) -> anyhow::Result<Stage> {
            let [a, b] = <[PlutusData; 2]>::try_from(fields)
                .map_err(|vec| anyhow!("expected 2 field, found {}", vec.len()))?;
            let pendings: Vec<Pending> = b
                .as_list()
                .ok_or(anyhow!("Expected list"))?
                .map(|x| Pending::try_from(&x))
                .collect::<anyhow::Result<Vec<Pending>>>()?;
            Ok(Stage::Responded(u64::try_from(&a)?, pendings))
        }
    }
}

impl<'a> From<Stage> for PlutusData<'a> {
    fn from(value: Stage) -> Self {
        match value {
            Stage::Opened(a, b) => constr!(0, a, PlutusData::list(b)),
            Stage::Closed(a, b, c) => constr!(1, a, PlutusData::list(b), c),
            Stage::Responded(a, b) => constr!(2, a, PlutusData::list(b)),
        }
    }
}
