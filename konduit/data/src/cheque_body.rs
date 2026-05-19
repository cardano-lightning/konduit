use crate::{Duration, Lock, Secret};
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

impl<C, T> minicbor::Encode<C> for ChequeBody<T>
where
    T: minicbor::Encode<C>,
{
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.begin_array()?;
        e.encode_with(self.index(), ctx)?;
        e.encode_with(self.amount(), ctx)?;
        e.encode_with(self.timeout(), ctx)?;
        e.encode_with(self.latch(), ctx)?;
        e.end()?;
        Ok(())
    }
}

impl<'b, C, T> minicbor::Decode<'b, C> for ChequeBody<T>
where
    T: minicbor::Decode<'b, C>,
{
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        d.array()?; // None = indef, Some(n) = definite — handle both or assert indef
        let index = d.decode_with(ctx)?;
        let amount = d.decode_with(ctx)?;
        let timeout: Duration = d.decode_with(ctx)?;
        let latch: T = d.decode_with(ctx)?;
        if d.datatype()? != minicbor::data::Type::Break {
            return Err(minicbor::decode::Error::message("expected end of array"));
        }
        d.skip()?;
        Ok(Self::new(index, amount, timeout, latch))
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
