use anyhow::anyhow;
use cardano_tx_builder::PlutusData;
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, fmt, str};

use crate::MAX_EXCLUDE_LENGTH;

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Indexes(pub Vec<u64>);

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
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let inner = s
            .split(",")
            .map(|x| x.trim())
            .filter(|x| !x.is_empty())
            .map(|x| x.parse::<u64>())
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self::new(inner)?)
    }
}

impl Indexes {
    pub fn empty() -> Self {
        Self(vec![])
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

impl<'a> TryFrom<PlutusData<'a>> for Indexes {
    type Error = anyhow::Error;

    fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
        let l = data
            .as_list()
            .ok_or(anyhow!("Expected list"))?
            .map(|x| x.as_integer().ok_or(anyhow!("Expected integer")))
            .collect::<anyhow::Result<Vec<u64>>>()?;
        let i = Self::new(l)?;
        Ok(i)
    }
}

impl<'a> From<&Indexes> for PlutusData<'a> {
    fn from(value: &Indexes) -> Self {
        Self::list(value.0.iter().map(|x| PlutusData::from(*x)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering;

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
