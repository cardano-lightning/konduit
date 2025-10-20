use crate::{Timestamp, Unlocked};
use cardano_tx_builder::PlutusData;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Unlockeds(pub Vec<Unlocked>);

impl Unlockeds {
    pub fn amount(&self) -> u64 {
        self.0.iter().map(|x| x.cheque_body.amount).sum()
    }

    pub fn max_timeout(&self) -> Option<Timestamp> {
        self.0
            .iter()
            .map(|x| x.cheque_body.timeout)
            .max()
            .map(|t| Timestamp(Duration::from_millis(t)))
    }
}

impl<'a> TryFrom<&PlutusData<'a>> for Unlockeds {
    type Error = anyhow::Error;

    fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
        let unlockeds: Vec<PlutusData<'_>> = Vec::try_from(data)?;
        let inner = unlockeds
            .iter()
            .map(Unlocked::try_from)
            .collect::<Result<Vec<Unlocked>, _>>()?;
        Ok(Unlockeds(inner))
    }
}

impl<'a> From<Unlockeds> for PlutusData<'a> {
    fn from(value: Unlockeds) -> Self {
        Self::list(value.0)
    }
}
