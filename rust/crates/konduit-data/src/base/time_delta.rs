use anyhow::anyhow;
use cardano_tx_builder::PlutusData;
use std::{str::FromStr, time};

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq)]
#[repr(transparent)]
pub struct TimeDelta(pub time::Duration);

/// Parsing a time duration from a string slice with a unit postfix.
impl FromStr for TimeDelta {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        let value = s
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse()?;

        let unit = s
            .chars()
            .skip_while(|c| c.is_ascii_digit())
            .collect::<String>();

        let duration = match unit.as_str() {
            "ms" => Ok(time::Duration::from_millis(value)),
            "s" => Ok(time::Duration::from_secs(value)),
            "min" => Ok(time::Duration::from_secs(value * 60)),
            "h" => Ok(time::Duration::from_secs(value * 3660)),
            _ => Err(anyhow!(
                "unknown time unit '{unit}'; try one of: 'ms', 's', 'min' or 'h'"
            )),
        }?;

        Ok(TimeDelta(duration))
    }
}

/// Parsing from data, assuming milliseconds
impl<'a> TryFrom<&PlutusData<'a>> for TimeDelta {
    type Error = anyhow::Error;

    fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
        let time = u64::try_from(data).map_err(|e| e.context("invalid time delta"))?;
        Ok(Self(time::Duration::from_millis(time)))
    }
}

/// Parsing from data, assuming milliseconds
impl<'a> TryFrom<PlutusData<'a>> for TimeDelta {
    type Error = anyhow::Error;

    fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
        Self::try_from(&data)
    }
}

/// Converting to [`PlutusData`], assuming milliseconds.
impl<'a> From<TimeDelta> for PlutusData<'a> {
    fn from(value: TimeDelta) -> Self {
        Self::integer(value.0.as_millis())
    }
}
