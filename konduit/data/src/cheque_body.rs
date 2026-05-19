use crate::{Duration, Lock, Secret};
use cardano_sdk::PlutusData;
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "cddl", derive(cuddly::ToCddl))]
#[cfg_attr(feature = "proptest", derive(proptest_derive::Arbitrary))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(bound = "T: Serialize + for<'de2> Deserialize<'de2>")]
pub struct ChequeBody<T = Lock> {
    #[cfg_attr(feature = "cddl", n(0))]
    index: u64,
    #[cfg_attr(feature = "cddl", n(1))]
    amount: u64,
    #[cfg_attr(feature = "cddl", n(2))]
    timeout: Duration,
    #[cfg_attr(feature = "cddl", n(3))]
    latch: T,
}

impl<T> ChequeBody<T> {
    pub fn new(index: u64, amount: u64, timeout: Duration, latch: T) -> Self {
        Self {
            index,
            amount,
            timeout,
            latch,
        }
    }

    pub fn index(&self) -> u64 {
        self.index
    }
    pub fn amount(&self) -> u64 {
        self.amount
    }
    pub fn timeout(&self) -> Duration {
        self.timeout
    }
    pub fn latch(&self) -> &T {
        &self.latch
    }
}

impl ChequeBody<Lock> {
    pub fn lock(&self) -> &Lock {
        self.latch()
    }

    pub fn is_secret(&self, secret: &Secret) -> bool {
        self.lock() == &Lock::from(secret)
    }

    pub fn try_unlocked(&self, secret: Secret) -> Result<ChequeBody<Secret>, &str> {
        if Lock::from(&secret) != *self.lock() {
            return Err("Bad secret");
        }
        Ok(ChequeBody::new(
            self.index(),
            self.amount(),
            self.timeout(),
            secret,
        ))
    }
}

impl ChequeBody<Secret> {
    pub fn secret(&self) -> Secret {
        *self.latch()
    }

    pub fn lock(&self) -> Lock {
        Lock::from(self.secret())
    }

    pub fn locked(&self) -> ChequeBody<Lock> {
        ChequeBody::new(self.index(), self.amount(), self.timeout(), self.lock())
    }
}

// FIXME :: Spent about an hour trying to get the generic version
// to encompass both Lock and Secret variants. Massive fail
impl<'a> TryFrom<PlutusData<'a>> for ChequeBody<Lock> {
    type Error = anyhow::Error;
    fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
        let [a, b, c, d] = <[PlutusData; 4]>::try_from(&data)?;
        Ok(Self::new(
            <u64>::try_from(&a)?,
            <u64>::try_from(&b)?,
            Duration::try_from(&c)?,
            Lock::try_from(&d)?,
        ))
    }
}

impl<'a> TryFrom<PlutusData<'a>> for ChequeBody<Secret> {
    type Error = anyhow::Error;
    fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
        let [a, b, c, d] = <[PlutusData; 4]>::try_from(&data)?;
        Ok(Self::new(
            <u64>::try_from(&a)?,
            <u64>::try_from(&b)?,
            Duration::try_from(&c)?,
            Secret::try_from(&d)?,
        ))
    }
}

impl<T> From<ChequeBody<T>> for PlutusData<'static>
where
    T: Clone,
    PlutusData<'static>: From<T> + From<u64> + From<Duration>,
{
    fn from(value: ChequeBody<T>) -> Self {
        Self::list([
            PlutusData::from(value.index()),
            PlutusData::from(value.amount()),
            PlutusData::from(value.timeout()),
            PlutusData::from(value.latch().clone()),
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
            latch: Lock([1; 32]),
        };

        let ser = serde_json::to_vec(&original).expect("Failed to serialize ChequeBody");
        let de: ChequeBody =
            serde_json::from_slice(&ser).expect("Failed to deserialize ChequeBody");

        assert_eq!(original, de);
    }
}
