use cardano_tx_builder::PlutusData;

use crate::pending::Pending;

#[derive(Debug, Clone)]
pub struct Pendings(pub Vec<Pending>);

impl<'a> TryFrom<&PlutusData<'a>> for Pendings {
    type Error = anyhow::Error;

    fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
        let list = Vec::try_from(data)?;
        let inner = list
            .iter()
            .map(Pending::try_from)
            .collect::<Result<Vec<Pending>, _>>()?;
        Ok(Pendings(inner))
    }
}

impl<'a> From<Pendings> for PlutusData<'a> {
    fn from(value: Pendings) -> Self {
        Self::list(value.0)
    }
}
