use crate::{Duration, Lock, Secret};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "proptest", derive(proptest_derive::Arbitrary))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(bound(
        serialize = "T: Serialize",
        deserialize = "T: for<'de2> Deserialize<'de2>",
    ))
)]
pub struct ChequeBody<T = Lock> {
    index: u64,
    amount: u64,
    timeout: Duration,
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
        d.array()?;
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

#[cfg(feature = "cardano_sdk")]
mod via_plutus_data {
    use super::*;
    use anyhow::anyhow;
    use cardano_sdk::PlutusData;

    impl<'a, T> From<ChequeBody<T>> for PlutusData<'a>
    where
        T: Into<PlutusData<'a>>,
    {
        fn from(body: ChequeBody<T>) -> Self {
            let items: Vec<PlutusData<'a>> = vec![
                PlutusData::from(body.index),
                PlutusData::from(body.amount),
                PlutusData::from(u64::from(body.timeout)),
                body.latch.into(),
            ];
            Self::list(items)
        }
    }

    impl<'a, T> TryFrom<PlutusData<'a>> for ChequeBody<T>
    where
        T: for<'b> TryFrom<PlutusData<'b>, Error = anyhow::Error>,
    {
        type Error = anyhow::Error;

        fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
            let items: Vec<PlutusData<'_>> = data
                .as_list()
                .ok_or(anyhow!("expected list for ChequeBody"))?
                .collect();
            let [index_pd, amount_pd, timeout_pd, latch_pd]: [PlutusData<'_>; 4] = items
                .try_into()
                .map_err(|_| anyhow!("expected 4 fields in ChequeBody"))?;
            let index: u64 = index_pd
                .as_integer()
                .ok_or(anyhow!("expected integer for index"))?;
            let amount: u64 = amount_pd
                .as_integer()
                .ok_or(anyhow!("expected integer for amount"))?;
            let timeout_ms: u64 = timeout_pd
                .as_integer()
                .ok_or(anyhow!("expected integer for timeout"))?;
            let latch = T::try_from(latch_pd)?;
            Ok(Self::new(
                index,
                amount,
                Duration::from_millis(timeout_ms),
                latch,
            ))
        }
    }
}

#[cfg(feature = "proptest")]
#[allow(unused_imports)]
mod roundtrip {
    use super::*;
    use cardano_sdk::{PlutusData, cbor::ToCbor};
    use proptest::prelude::*;

    proptest! {
        /// minicbor encodes and decodes ChequeBody back to the same value.
        #[test]
        fn cbor(val: ChequeBody) {
            let bytes = minicbor::to_vec(&val).unwrap();
            let recovered: ChequeBody = minicbor::decode(&bytes).unwrap();
            prop_assert_eq!(val, recovered);
        }

        /// minicbor bytes are byte-for-byte identical to PlutusData's canonical CBOR.
        #[test]
        fn encoding_matches(val: ChequeBody) {
            let mini = minicbor::to_vec(&val).unwrap();
            let pd = PlutusData::from(val).to_cbor();
            prop_assert_eq!(mini, pd);
        }

        /// PlutusData's canonical CBOR decodes via minicbor back to the same value.
        #[test]
        fn from_plutus(val: ChequeBody) {
            let pd_bytes = PlutusData::from(val.clone()).to_cbor();
            let recovered: ChequeBody = minicbor::decode(&pd_bytes).unwrap();
            prop_assert_eq!(val, recovered);
        }

        /// From<ChequeBody> for PlutusData and TryFrom<PlutusData> for ChequeBody are mutual inverses.
        #[test]
        fn tryfrom(val: ChequeBody) {
            let pd = PlutusData::from(val.clone());
            let recovered: ChequeBody = ChequeBody::try_from(pd).unwrap();
            prop_assert_eq!(val, recovered);
        }
    }
}

#[cfg(test)]
#[cfg(all(test, feature = "json"))]
mod json_tests {
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
