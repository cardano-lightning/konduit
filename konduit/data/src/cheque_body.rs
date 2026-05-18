use crate::{Duration, Lock, Secret};
use anyhow::{Error, Result};
use cardano_sdk::PlutusData;
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "cddl", derive(cuddly::ToCddl))]
#[cfg_attr(feature = "proptest", derive(proptest_derive::Arbitrary))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ChequeBody {
    #[cfg_attr(feature = "cddl", n(0))]
    pub index: u64,
    #[cfg_attr(feature = "cddl", n(1))]
    pub amount: u64,
    #[cfg_attr(feature = "cddl", n(2))]
    pub timeout: Duration,
    #[cfg_attr(feature = "cddl", n(3))]
    pub lock: Lock,
}

impl ChequeBody {
    pub fn new(index: u64, amount: u64, timeout: Duration, lock: Lock) -> Self {
        Self {
            index,
            amount,
            timeout,
            lock,
        }
    }

    pub fn is_secret(&self, secret: &Secret) -> bool {
        self.lock == Lock::from(secret.clone())
    }
}

impl<'a> TryFrom<PlutusData<'a>> for ChequeBody {
    type Error = Error;

    fn try_from(data: PlutusData<'a>) -> Result<Self> {
        let [a, b, c, d] = <[PlutusData; 4]>::try_from(&data)?;
        Ok(Self::new(
            <u64>::try_from(&a)?,
            <u64>::try_from(&b)?,
            Duration::try_from(&c)?,
            Lock::try_from(&d)?,
        ))
    }
}

impl<'a> From<ChequeBody> for PlutusData<'a> {
    fn from(value: ChequeBody) -> Self {
        Self::list([
            PlutusData::from(value.index),
            PlutusData::from(value.amount),
            PlutusData::from(value.timeout),
            PlutusData::from(value.lock),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cheque_body_round_trip() {
        let original = ChequeBody {
            index: 42,
            amount: 5000000,
            timeout: Duration::from_secs(123002302),
            lock: Lock([1; 32]),
        };

        let ser = serde_json::to_vec(&original).expect("Failed to serialize ChequeBody");
        let de: ChequeBody =
            serde_json::from_slice(&ser).expect("Failed to deserialize ChequeBody");

        assert_eq!(original, de);
    }
}
