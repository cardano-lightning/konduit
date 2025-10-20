use crate::{Lock, Pending, Secret, Timestamp, Unpends};
use cardano_tx_builder::PlutusData;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct Pendings(pub Vec<Pending>);

impl<'a> TryFrom<&PlutusData<'a>> for Pendings {
    type Error = anyhow::Error;

    fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
        let list = Vec::try_from(data)?;
        let inner = list
            .iter()
            .map(Pending::try_from)
            .collect::<Result<Vec<Pending>, _>>()?;
        Ok(Pendings(inner))
    }
}

impl<'a> From<Pendings> for PlutusData<'a> {
    fn from(value: Pendings) -> Self {
        Self::list(value.0)
    }
}

impl Pendings {
    pub fn amount(&self) -> u64 {
        self.0.iter().map(|x| x.amount).sum()
    }

    pub fn unlock(&self, secrets: Vec<Secret>, upper_bound: Timestamp) -> (Unpends, u64, Self) {
        let known: BTreeMap<Lock, Secret> = secrets
            .into_iter()
            .map(|s| (Lock::from_secret(s), s))
            .collect();
        let unpends: Vec<Vec<u8>> = self
            .0
            .iter()
            .map(|p| {
                if p.timeout.0 < upper_bound.0 {
                    match known.get(&p.lock) {
                        None => vec![],
                        Some(Secret(inner)) => inner.to_vec(),
                    }
                } else {
                    vec![]
                }
            })
            .collect();
        let release: u64 = self
            .0
            .iter()
            .zip(unpends.iter())
            .filter_map(|(p, s)| if s.is_empty() { None } else { Some(p.amount) })
            .sum();
        let remain: Vec<Pending> = self
            .0
            .iter()
            .zip(unpends.iter())
            .filter_map(|(p, s)| if s.is_empty() { Some(p.clone()) } else { None })
            .collect();
        (Unpends(unpends), release, Self(remain))
    }

    pub fn expire(&self, lower_bound: Timestamp) -> (Unpends, u64, Pendings) {
        let unpends: Vec<Vec<u8>> = self
            .0
            .iter()
            .map(|p| {
                if p.timeout.0 > lower_bound.0 {
                    vec![255]
                } else {
                    vec![]
                }
            })
            .collect();
        let release: u64 = self
            .0
            .iter()
            .zip(unpends.iter())
            .filter_map(|(p, s)| if s.is_empty() { None } else { Some(p.amount) })
            .sum();
        let remain: Vec<Pending> = self
            .0
            .iter()
            .zip(unpends.iter())
            .filter_map(|(p, s)| if s.is_empty() { Some(p.clone()) } else { None })
            .collect();
        (Unpends(unpends), release, Pendings(remain))
    }
}
