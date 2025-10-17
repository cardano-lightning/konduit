use cardano_tx_builder::PlutusData;

use crate::unlocked::Unlocked;

#[derive(Debug, Clone)]
pub struct Unlockeds(pub Vec<Unlocked>);

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
