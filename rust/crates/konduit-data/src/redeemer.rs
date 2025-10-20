use crate::Steps;
use anyhow::anyhow;
use cardano_tx_builder::{PlutusData, constr};

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
    type Error = anyhow::Error;

    fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
        let (variant, fields): (u64, Vec<PlutusData<'_>>) = (&data).try_into()?;

        return match variant {
            _ if variant == 0 => {
                try_batch(fields).map_err(|e| e.context("invalid 'Batch' variant"))
            }
            _ if variant == 1 => try_main(fields).map_err(|e| e.context("invalid 'Main' variant")),
            _ if variant == 2 => {
                try_mutual(fields).map_err(|e| e.context("invalid 'Mutual' variant"))
            }
            _ => Err(anyhow!("unknown variant: {variant}")),
        };

        fn try_batch(fields: Vec<PlutusData<'_>>) -> anyhow::Result<Redeemer> {
            let [] = <[PlutusData; 0]>::try_from(fields)
                .map_err(|vec| anyhow!("expected no fields, found {}", vec.len()))?;
            Ok(Redeemer::new_batch())
        }

        fn try_main(fields: Vec<PlutusData<'_>>) -> anyhow::Result<Redeemer> {
            let [a] = <[PlutusData; 1]>::try_from(fields)
                .map_err(|vec| anyhow!("expected 1 field, found {}", vec.len()))?;
            Ok(Redeemer::new_main(Steps::try_from(&a)?))
        }

        fn try_mutual(fields: Vec<PlutusData<'_>>) -> anyhow::Result<Redeemer> {
            let [] = <[PlutusData; 0]>::try_from(fields)
                .map_err(|vec| anyhow!("expected no fields, found {}", vec.len()))?;
            Ok(Redeemer::new_mutual())
        }
    }
}

impl<'a> From<Redeemer> for PlutusData<'a> {
    fn from(value: Redeemer) -> Self {
        match value {
            Redeemer::Batch => constr!(0),
            Redeemer::Main(steps) => constr!(1, steps),
            Redeemer::Mutual => constr!(2),
        }
    }
}
