use anyhow::anyhow;
use cardano_tx_builder::PlutusData;
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    ops::{Deref, DerefMut},
    str::FromStr,
    time,
};

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Duration(pub time::Duration);

impl Duration {
    pub fn from_secs(secs: u64) -> Self {
        Self(time::Duration::from_secs(secs))
    }

    pub fn from_millis(millis: u64) -> Self {
        Self(time::Duration::from_millis(millis))
    }
}

impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}ms", self.as_millis())
    }
}

/// Provide a 'Deref' instance so that we can easily call onto time::Duration methods without
/// having to perform any explicit conversions.
impl Deref for Duration {
    type Target = time::Duration;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Duration {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Parsing a time duration from a string slice with a unit postfix.
impl FromStr for Duration {
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

        Ok(Duration(duration))
    }
}

/// Parsing from data, assuming milliseconds
impl<'a> TryFrom<&PlutusData<'a>> for Duration {
    type Error = anyhow::Error;

    fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
        let time = u64::try_from(data).map_err(|e| e.context("invalid duration"))?;
        Ok(Self(time::Duration::from_millis(time)))
    }
}

/// Parsing from data, assuming milliseconds
impl<'a> TryFrom<PlutusData<'a>> for Duration {
    type Error = anyhow::Error;

    fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
        Self::try_from(&data)
    }
}

/// Converting to [`PlutusData`], assuming milliseconds.
impl<'a> From<Duration> for PlutusData<'a> {
    fn from(value: Duration) -> Self {
        Self::integer(value.0.as_millis())
    }
}

/// Converting to `u64`, assuming milliseconds.
impl From<&Duration> for u64 {
    fn from(value: &Duration) -> Self {
        value.0.as_millis() as u64
    }
}

/// Converting to `u64`, assuming milliseconds.
impl From<Duration> for u64 {
    fn from(value: Duration) -> Self {
        value.0.as_millis() as u64
    }
}
