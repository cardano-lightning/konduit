use anyhow::anyhow;
use cardano_tx_builder::PlutusData;

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct Amount(pub u64);

impl<'a> TryFrom<PlutusData<'a>> for Amount {
    type Error = anyhow::Error;

    fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
        Self::try_from(&data)
    }
}

impl<'a> TryFrom<&PlutusData<'a>> for Amount {
    type Error = anyhow::Error;

    fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
        let amount = u64::try_from(data).map_err(|e| e.context("invalid amount"))?;
        Ok(Self(amount))
    }
}

impl<'a> From<Amount> for PlutusData<'a> {
    fn from(value: Amount) -> Self {
        Self::integer(value.0)
    }
}

impl Amount {
    pub fn add(&self, x: u64) -> Self {
        Self(self.0 + x)
    }

    pub fn checked_sub(&self, x: u64) -> anyhow::Result<Self> {
        self.0
            .checked_sub(x)
            .map(Self)
            .ok_or(anyhow!("underflow: cannot subtract {x} from {}", self.0))
    }
}
