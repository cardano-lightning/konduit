use anyhow::anyhow;
use cardano_tx_builder::{PlutusData, constr};

#[derive(Debug, Clone)]
pub enum Stage {
    Opened(u64),
}

impl<'a> TryFrom<PlutusData<'a>> for Stage {
    type Error = anyhow::Error;

    fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
        let (variant, fields): (u64, Vec<PlutusData<'_>>) = (&data).try_into()?;

        return match variant {
            _ if variant == 0 => {
                try_opened(fields).map_err(|e| e.context("invalid 'Opened' variant"))
            }
            _ => Err(anyhow!("unknown variant: {variant}")),
        };

        fn try_opened(fields: Vec<PlutusData<'_>>) -> anyhow::Result<Stage> {
            let [a] = <[PlutusData; 1]>::try_from(fields)
                .map_err(|vec| anyhow!("expected 1 field, found {}", vec.len()))?;

            Ok(Stage::Opened(u64::try_from(&a)?))
        }
    }
}

impl<'a> From<Stage> for PlutusData<'a> {
    fn from(value: Stage) -> Self {
        match value {
            Stage::Opened(a) => constr!(0, a),
        }
    }
}
