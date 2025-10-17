use cardano_tx_builder::PlutusData;

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct Index(pub u64);

impl<'a> TryFrom<&PlutusData<'a>> for Index {
    type Error = anyhow::Error;

    fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
        let ix = u64::try_from(data).map_err(|e| e.context("invalid Index"))?;
        Ok(Self(ix))
    }
}

impl<'a> TryFrom<PlutusData<'a>> for Index {
    type Error = anyhow::Error;

    fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
        Self::try_from(&data)
    }
}

impl<'a> From<Index> for PlutusData<'a> {
    fn from(value: Index) -> Self {
        Self::integer(value.0)
    }
}

impl Index {
    pub fn incr(&self) -> Self {
        Self(self.0 + 1)
    }
}
