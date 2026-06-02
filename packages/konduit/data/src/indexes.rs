use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, fmt, str};

use crate::{MAX_EXCLUDE_LENGTH, ParseError};

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum IndexesError {
    #[error("Exceeds max allowed length")]
    Length,
    #[error("Less than last")]
    LessThanLast,
    #[error("Ordering Error")]
    Order,
    #[error("Duplicate Error")]
    Duplicate,
    #[error("Attempted to remove item not here")]
    NoIndex,
}

/// A sorted, deduplicated, bounded list of u64 indexes.
///
/// On-chain encoding: a CBOR indefinite-length array of uint items.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Indexes(pub(crate) Vec<u64>);

impl fmt::Display for Indexes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            self.0
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join(",")
        )?;
        Ok(())
    }
}

impl str::FromStr for Indexes {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let inner = s
            .split(",")
            .map(|x| x.trim())
            .filter(|x| !x.is_empty())
            .map(|x| x.parse::<u64>())
            .collect::<Result<Vec<_>, _>>()?;
        Self::new(inner).map_err(|e| ParseError::Constraint(e.to_string()))
    }
}

impl Indexes {
    pub fn empty() -> Self {
        Self(vec![])
    }

    pub fn last(&self) -> Option<u64> {
        self.0.last().copied()
    }

    pub fn new(items: Vec<u64>) -> Result<Self, IndexesError> {
        if items.len() > MAX_EXCLUDE_LENGTH {
            return Err(IndexesError::Length);
        }
        for window in items.windows(2) {
            if window[0] >= window[1] {
                return Err(IndexesError::Order);
            }
        }
        Ok(Self(items))
    }

    pub fn extend(&mut self, from: u64, until: u64) -> Result<(), IndexesError> {
        if until < from {
            return Err(IndexesError::Order);
        }
        if self.0.len() + (until - from) as usize > MAX_EXCLUDE_LENGTH {
            return Err(IndexesError::Length);
        }
        let range = from..until;
        if let Some(last) = self.0.last() {
            match last < &from {
                true => {
                    let _: () = self.0.extend(range);
                    Ok(())
                }
                false => Err(IndexesError::LessThanLast),
            }
        } else {
            self.0.extend(range);
            Ok(())
        }
    }

    pub fn insert(&mut self, item: u64) -> Result<(), IndexesError> {
        if self.0.len() >= MAX_EXCLUDE_LENGTH {
            return Err(IndexesError::Length);
        }
        match self.0.binary_search(&item) {
            Ok(_) => Err(IndexesError::Duplicate),
            Err(index) => {
                self.0.insert(index, item);
                Ok(())
            }
        }
    }

    pub fn remove(&mut self, item: u64) -> Result<(), IndexesError> {
        let Ok(index) = self.0.binary_search(&item) else {
            return Err(IndexesError::NoIndex);
        };
        self.0.remove(index);
        Ok(())
    }

    pub fn has(&self, item: u64) -> bool {
        self.0.binary_search(&item).is_ok()
    }
}

impl PartialOrd for Indexes {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let self_is_superset = is_superset_of(&self.0, &other.0);
        let other_is_superset = is_superset_of(&other.0, &self.0);
        match (self_is_superset, other_is_superset) {
            (true, true) => Some(Ordering::Equal),
            (true, false) => Some(Ordering::Less),
            (false, true) => Some(Ordering::Greater),
            (false, false) => None,
        }
    }
}

fn is_superset_of(a: &[u64], b: &[u64]) -> bool {
    let mut b_iter = b.iter().peekable();
    for a_item in a {
        let Some(b_item) = b_iter.peek() else {
            return true;
        };
        if a_item == *b_item {
            b_iter.next();
        } else if a_item > *b_item {
            return false;
        }
    }
    b_iter.peek().is_none()
}

impl<C> minicbor::Encode<C> for Indexes {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        crate::cbor_with::plutus_list::encode(&self.0, e, ctx)
    }
}

impl<'b, C> minicbor::Decode<'b, C> for Indexes {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let items: Vec<u64> = crate::cbor_with::plutus_list::decode(d, ctx)?;
        Self::new(items).map_err(|e| minicbor::decode::Error::message(e.to_string()))
    }
}

#[cfg(feature = "proptest")]
impl proptest::arbitrary::Arbitrary for Indexes {
    type Parameters = ();
    type Strategy = proptest::strategy::BoxedStrategy<Self>;
    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        proptest::collection::btree_set(any::<u64>(), 0..=crate::MAX_EXCLUDE_LENGTH)
            .prop_map(|set| Indexes(set.into_iter().collect()))
            .boxed()
    }
}

#[cfg(feature = "cardano_sdk")]
mod via_plutus_data {
    use super::*;
    use anyhow::anyhow;
    use cardano_sdk::PlutusData;

    impl<'a> TryFrom<PlutusData<'a>> for Indexes {
        type Error = anyhow::Error;

        fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
            let l = data
                .as_list()
                .ok_or(anyhow!("Expected list"))?
                .map(|x| x.as_integer().ok_or(anyhow!("Expected integer")))
                .collect::<anyhow::Result<Vec<u64>>>()?;
            Ok(Self::new(l)?)
        }
    }

    impl<'a> From<&Indexes> for PlutusData<'a> {
        fn from(value: &Indexes) -> Self {
            Self::list(value.0.iter().map(|x| PlutusData::from(*x)))
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
        /// minicbor encodes and decodes Indexes back to the same value.
        #[test]
        fn cbor(val: Indexes) {
            let bytes = minicbor::to_vec(&val).unwrap();
            let recovered: Indexes = minicbor::decode(&bytes).unwrap();
            prop_assert_eq!(val, recovered);
        }

        /// minicbor bytes are byte-for-byte identical to PlutusData's canonical CBOR.
        #[test]
        fn encoding_matches(val: Indexes) {
            let mini = minicbor::to_vec(&val).unwrap();
            let pd = PlutusData::from(&val).to_cbor();
            prop_assert_eq!(mini, pd);
        }

        /// PlutusData's canonical CBOR decodes via minicbor back to the same value.
        #[test]
        fn from_plutus(val: Indexes) {
            let pd_bytes = PlutusData::from(&val).to_cbor();
            let recovered: Indexes = minicbor::decode(&pd_bytes).unwrap();
            prop_assert_eq!(val, recovered);
        }

        /// From<&Indexes> for PlutusData and TryFrom<PlutusData> for Indexes are mutual inverses.
        #[test]
        fn tryfrom(val: Indexes) {
            let pd = PlutusData::from(&val);
            let recovered = Indexes::try_from(pd).unwrap();
            prop_assert_eq!(val, recovered);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering;

    #[test]
    fn indexes_cbor_roundtrip_deterministic() {
        let idx = Indexes::new(vec![1, 5, 9]).unwrap();
        let bytes = minicbor::to_vec(&idx).unwrap();
        let recovered: Indexes = minicbor::decode(&bytes).unwrap();
        assert_eq!(idx, recovered);
    }

    #[test]
    fn test_partial_ord_less() {
        let u = Indexes::new(vec![1, 2, 3, 4, 5]).unwrap();
        let v = Indexes::new(vec![2, 4]).unwrap();
        assert_eq!(u.partial_cmp(&v), Some(Ordering::Less));
        assert!(u < v);
    }

    #[test]
    fn test_partial_ord_greater() {
        let u = Indexes::new(vec![1, 2, 3, 4, 5]).unwrap();
        let v = Indexes::new(vec![2, 4]).unwrap();
        assert_eq!(v.partial_cmp(&u), Some(Ordering::Greater));
        assert!(v > u);
    }

    #[test]
    fn test_partial_ord_equal() {
        let a = Indexes::new(vec![10, 20, 30]).unwrap();
        let b = Indexes::new(vec![10, 20, 30]).unwrap();
        assert_eq!(a.partial_cmp(&b), Some(Ordering::Equal));
        assert!(a == b);
    }

    #[test]
    fn test_partial_ord_none() {
        let a = Indexes::new(vec![1, 2, 3]).unwrap();
        let b = Indexes::new(vec![2, 3, 4]).unwrap();
        assert_eq!(a.partial_cmp(&b), None);
    }

    #[test]
    fn test_partial_ord_empty() {
        let a = Indexes::new(vec![1, 2, 3]).unwrap();
        let b_empty = Indexes::new(vec![]).unwrap();
        assert_eq!(a.partial_cmp(&b_empty), Some(Ordering::Less));
        assert!(a < b_empty);
        assert_eq!(b_empty.partial_cmp(&a), Some(Ordering::Greater));
        assert!(b_empty > a);
    }

    #[test]
    fn test_partial_ord_both_empty() {
        let a_empty = Indexes::new(vec![]).unwrap();
        let b_empty = Indexes::new(vec![]).unwrap();
        assert_eq!(a_empty.partial_cmp(&b_empty), Some(Ordering::Equal));
        assert!(a_empty == b_empty);
    }
}
