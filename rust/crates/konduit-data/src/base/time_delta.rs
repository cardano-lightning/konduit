use cardano_tx_builder::PlutusData;

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct TimeDelta(pub u64);

impl<'a> TryFrom<&PlutusData<'a>> for TimeDelta {
    type Error = anyhow::Error;

    fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
        let time = u64::try_from(data).map_err(|e| e.context("invalid time delta"))?;
        Ok(Self(time))
    }
}

impl<'a> TryFrom<PlutusData<'a>> for TimeDelta {
    type Error = anyhow::Error;

    fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
        Self::try_from(&data)
    }
}

impl<'a> From<TimeDelta> for PlutusData<'a> {
    fn from(value: TimeDelta) -> Self {
        Self::integer(value.0)
    }
}
