use crate::ParseError;
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    ops::{Add, Deref, DerefMut},
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
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, ParseError> {
        let value: u64 = s
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse()?;

        let unit = s
            .chars()
            .skip_while(|c| c.is_ascii_digit())
            .collect::<String>();

        let duration = match unit.as_str() {
            "ms" => time::Duration::from_millis(value),
            "s" => time::Duration::from_secs(value),
            "min" => time::Duration::from_secs(value * 60),
            "h" => time::Duration::from_secs(value * 3600),
            _ => {
                return Err(ParseError::Constraint(format!(
                    "unknown time unit '{unit}'; try one of: 'ms', 's', 'min' or 'h'"
                )));
            }
        };

        Ok(Duration(duration))
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

impl Add for Duration {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        // Accessing .0 directly or using deref
        Duration(self.0 + rhs.0)
    }
}

impl<C> minicbor::Encode<C> for Duration {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.encode_with(self.0.as_millis() as u64, ctx)?;
        Ok(())
    }
}

impl<'b, C> minicbor::Decode<'b, C> for Duration {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let millis: u64 = d.decode_with(ctx)?;
        Ok(Self(time::Duration::from_millis(millis)))
    }
}

#[cfg(feature = "proptest")]
impl proptest::arbitrary::Arbitrary for Duration {
    type Parameters = ();
    type Strategy = proptest::strategy::BoxedStrategy<Self>;
    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        any::<u64>().prop_map(Duration::from_millis).boxed()
    }
}

#[cfg(feature = "cardano_sdk")]
mod via_plutus_data {
    use super::*;
    use cardano_sdk::PlutusData;

    impl<'a> From<Duration> for PlutusData<'a> {
        fn from(value: Duration) -> Self {
            PlutusData::from(u64::from(value))
        }
    }

    impl<'a> TryFrom<&PlutusData<'a>> for Duration {
        type Error = anyhow::Error;
        fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
            Ok(Self::from_millis(u64::try_from(data)?))
        }
    }

    impl<'a> TryFrom<PlutusData<'a>> for Duration {
        type Error = anyhow::Error;
        fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
            Self::try_from(&data)
        }
    }
}

#[cfg(feature = "proptest")]
#[allow(unused_imports)]
mod roundtrip {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// minicbor encodes and decodes Duration back to the same value.
        #[test]
        fn duration_cbor_roundtrip(val: Duration) {
            let bytes = minicbor::to_vec(&val).unwrap();
            let recovered: Duration = minicbor::decode(&bytes).unwrap();
            prop_assert_eq!(val, recovered);
        }

        /// Duration is encoded as its millisecond count — a plain u64.
        #[test]
        fn duration_encodes_as_millis(val: Duration) {
            let mini = minicbor::to_vec(&val).unwrap();
            let millis = val.as_millis() as u64;
            let expected = minicbor::to_vec(&millis).unwrap();
            prop_assert_eq!(mini, expected);
        }
    }
}
