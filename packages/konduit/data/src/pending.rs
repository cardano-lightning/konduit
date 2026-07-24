use crate::{Duration, Lock, Locked, VerifyState};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "proptest", derive(proptest_derive::Arbitrary))]
#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Pending {
    pub amount: u64,
    pub timeout: Duration,
    pub lock: Lock,
}

impl Pending {
    pub fn new(amount: u64, timeout: Duration, lock: Lock) -> Self {
        Self {
            amount,
            timeout,
            lock,
        }
    }
}

impl<V: VerifyState> From<Locked<V>> for Pending {
    fn from(value: Locked<V>) -> Self {
        Self {
            amount: value.amount(),
            timeout: value.timeout(),
            lock: *value.lock(),
        }
    }
}

/// On-chain encoding: an indefinite-length array of [amount, timeout, lock].
impl<C> minicbor::Encode<C> for Pending {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.begin_array()?;
        e.encode_with(self.amount, ctx)?;
        e.encode_with(self.timeout, ctx)?;
        e.encode_with(self.lock, ctx)?;
        e.end()?;
        Ok(())
    }
}

impl<'b, C> minicbor::Decode<'b, C> for Pending {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        d.array()?;
        let amount: u64 = d.decode_with(ctx)?;
        let timeout: Duration = d.decode_with(ctx)?;
        let lock: Lock = d.decode_with(ctx)?;
        if d.datatype()? != minicbor::data::Type::Break {
            return Err(minicbor::decode::Error::message(
                "expected end of Pending array",
            ));
        }
        d.skip()?;
        Ok(Self::new(amount, timeout, lock))
    }
}

#[cfg(feature = "cardano_sdk")]
mod via_plutus_data {
    use super::*;
    use anyhow::anyhow;
    use cardano_sdk::PlutusData;

    impl<'a> TryFrom<Vec<PlutusData<'a>>> for Pending {
        type Error = anyhow::Error;

        fn try_from(list: Vec<PlutusData<'a>>) -> anyhow::Result<Self> {
            let [a, b, c] = <[PlutusData; 3]>::try_from(list)
                .map_err(|_| anyhow!("invalid 'Pending': expected 3-element list"))?;
            Ok(Self::new(
                u64::try_from(&a)?,
                Duration::try_from(b)?,
                Lock::try_from(c)?,
            ))
        }
    }

    impl<'a> TryFrom<&PlutusData<'a>> for Pending {
        type Error = anyhow::Error;

        fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
            Self::try_from(Vec::try_from(data)?)
        }
    }

    impl<'a> TryFrom<PlutusData<'a>> for Pending {
        type Error = anyhow::Error;

        fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
            Self::try_from(&data)
        }
    }

    impl<'a> From<Pending> for PlutusData<'a> {
        fn from(pending: Pending) -> Self {
            Self::list(vec![
                PlutusData::from(pending.amount),
                PlutusData::from(pending.timeout),
                PlutusData::from(pending.lock),
            ])
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
        /// minicbor encodes and decodes Pending back to the same value.
        #[test]
        fn cbor(val: Pending) {
            let bytes = minicbor::to_vec(&val).unwrap();
            let recovered: Pending = minicbor::decode(&bytes).unwrap();
            prop_assert_eq!(val, recovered);
        }

        /// minicbor bytes are byte-for-byte identical to PlutusData's canonical CBOR.
        #[test]
        fn encoding_matches(val: Pending) {
            let mini = minicbor::to_vec(&val).unwrap();
            let pd = PlutusData::from(val).to_cbor();
            prop_assert_eq!(mini, pd);
        }

        /// PlutusData's canonical CBOR decodes via minicbor back to the same value.
        #[test]
        fn from_plutus(val: Pending) {
            let pd_bytes = PlutusData::from(val.clone()).to_cbor();
            let recovered: Pending = minicbor::decode(&pd_bytes).unwrap();
            prop_assert_eq!(val, recovered);
        }

        /// From<Pending> for PlutusData and TryFrom<PlutusData> for Pending are mutual inverses.
        #[test]
        fn tryfrom(val: Pending) {
            let pd = PlutusData::from(val.clone());
            let recovered = Pending::try_from(pd).unwrap();
            prop_assert_eq!(val, recovered);
        }
    }
}
