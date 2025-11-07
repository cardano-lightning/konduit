use anyhow::anyhow;

use cardano_tx_builder::{PlutusData, VerificationKey};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    Cheque, Duration, Indexes, Lock, MAX_UNSQUASHED, MixedCheque, Receipt, Secret, Squash,
    SquashBody, SquashBodySquashError, Tag, Unlocked, plutus_data_serde,
};

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum MixedReceiptUpdateError {
    #[error("Squash cannot include a (locked) cheque.")]
    IncludesCheque,

    #[error("Squash body error {0}")]
    SquashBody(SquashBodySquashError),

    #[error("Squash body was not reproduced")]
    NotReproduced,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MixedReceipt {
    pub squash: Squash,
    pub mixed_cheques: Vec<MixedCheque>,
}

impl Serialize for MixedReceipt {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        plutus_data_serde::serialize(self, serializer)
    }
}

impl<'de> Deserialize<'de> for MixedReceipt {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        plutus_data_serde::deserialize::<D, Self>(deserializer)
    }
}

impl MixedReceipt {
    pub fn new(squash: Squash, mixed_cheques: Vec<MixedCheque>) -> anyhow::Result<Self> {
        if mixed_cheques.len() > MAX_UNSQUASHED {
            Err(anyhow!("Too many unsquashed"))?;
        }
        let mut sorted: Vec<MixedCheque> = vec![];
        for mixed_cheque in mixed_cheques {
            let index = mixed_cheque.index();
            if squash.squash_body.is_index_squashed(index) {
                Err(anyhow!("Index {} is already squashed", index))?;
            }
            match sorted.binary_search_by(|probe| probe.index().cmp(&index)) {
                Ok(_) => Err(anyhow!("Duplicate index {}", index))?,
                Err(position) => sorted.insert(position, mixed_cheque),
            };
        }
        sorted.sort();
        Ok(Self::new_no_verify(squash, sorted))
    }

    pub fn new_no_verify(squash: Squash, mixed_cheques: Vec<MixedCheque>) -> Self {
        Self {
            squash,
            mixed_cheques,
        }
    }

    pub fn max_index(&self) -> u64 {
        let squash_index = self.squash.squash_body.index;
        match self.mixed_cheques.last() {
            Some(mc) => std::cmp::max(squash_index, mc.index()),
            None => squash_index,
        }
    }
    pub fn verify_components(&self, key: &VerificationKey, tag: &Tag) -> bool {
        self.squash.verify(key, tag)
            && self.mixed_cheques.iter().all(|m| m.verify(key, tag))
            && match Self::new(self.squash.clone(), self.mixed_cheques.clone()) {
                Ok(copy) => self == &copy,
                Err(_) => false,
            }
    }

    pub fn capacity(&self) -> usize {
        MAX_UNSQUASHED - self.mixed_cheques.len()
    }

    pub fn cheques(&self) -> Vec<Cheque> {
        self.mixed_cheques
            .iter()
            .filter_map(|x| x.as_cheque())
            .collect::<Vec<Cheque>>()
    }

    pub fn unlockeds(&self) -> Vec<Unlocked> {
        self.mixed_cheques
            .iter()
            .filter_map(|x| x.as_unlocked())
            .collect::<Vec<Unlocked>>()
    }

    pub fn unlock(&mut self, secret: Secret) {
        let lock = Lock::from(secret.clone());
        self.mixed_cheques.iter_mut().for_each(|i| {
            if let MixedCheque::Cheque(cheque) = i
                && lock == cheque.cheque_body.lock
            {
                *i = MixedCheque::Unlocked(Unlocked::new(cheque.clone(), secret.clone()).unwrap());
            }
        })
    }

    pub fn receipt(&self) -> Receipt {
        Receipt::new_no_verify(self.squash.clone(), self.unlockeds())
    }

    pub fn amount(&self) -> u64 {
        self.squash.amount()
            + self
                .unlockeds()
                .iter()
                .map(|x| x.cheque_body.amount)
                .sum::<u64>()
    }

    pub fn committed(&self) -> u64 {
        self.squash.amount() + self.mixed_cheques.iter().map(|x| x.amount()).sum::<u64>()
    }

    /// Time and signature must already be verified
    pub fn insert(&mut self, cheque: Cheque) -> anyhow::Result<()> {
        let index = cheque.cheque_body.index;
        let mixed_cheque = MixedCheque::from(cheque);
        match self
            .mixed_cheques
            .binary_search_by(|probe| probe.index().cmp(&index))
        {
            Ok(_) => Err(anyhow!("Duplicate index {}", &index))?,
            Err(position) => self.mixed_cheques.insert(position, mixed_cheque),
        };
        Ok(())
    }

    /// Collect all cheques that have timed out relative to timeout.
    /// These can be safely expired in a squash.
    pub fn timed_out(&mut self, timeout: Duration) -> Indexes {
        let inner = self
            .mixed_cheques
            .iter()
            .filter_map(|mixed_cheque| match mixed_cheque {
                MixedCheque::Unlocked(_) => None,
                MixedCheque::Cheque(cheque) => {
                    if cheque.cheque_body.timeout < timeout {
                        Some(cheque.cheque_body.index)
                    } else {
                        None
                    }
                }
            })
            .collect::<Vec<u64>>();
        Indexes::new(inner).expect("Impossible")
    }

    /// Collect all cheques that have timed out relative to timeout.
    /// These can be safely expired in a squash.
    pub fn expire(&mut self, indexes: Indexes) -> anyhow::Result<()> {
        let curr = Indexes::new(
            self.cheques()
                .iter()
                .map(|x| x.cheque_body.index)
                .collect::<Vec<u64>>(),
        )?;
        if curr >= indexes {
            Err(anyhow!("Indexes must be a subset of cheque indexes"))
        } else {
            self.mixed_cheques
                .retain(|mixed_cheque| match mixed_cheque {
                    MixedCheque::Unlocked(_) => true,
                    MixedCheque::Cheque(cheque) => indexes.has(cheque.cheque_body.index),
                });
            Ok(())
        }
    }

    /// Time and signature must already be verified since these need
    /// verification key and tag that are not present in the mixed receipt.
    ///
    /// Squash must be the result of the previous squash and some known unlockeds.
    /// No cheque on the receipt can be squashed.
    /// Any timed_out cheque must first be expired.
    /// Expired cheques must be expired first.
    /// The return is a result.
    /// If update is ok and complete, the return is `Ok(true)`.
    /// If update is ok yet incomplete, the return is `Ok(false)`.
    /// Otherwise the update is `Err`.
    pub fn update(&mut self, squash: Squash) -> Result<bool, MixedReceiptUpdateError> {
        let mut current = self.squash.squash_body.clone();
        let proposed = &squash.squash_body;
        for mixed_cheque in &self.mixed_cheques {
            match mixed_cheque {
                MixedCheque::Unlocked(unlocked) => {
                    if proposed.is_index_squashed(unlocked.cheque_body.index) {
                        current
                            .squash(unlocked.cheque_body.clone())
                            .map_err(MixedReceiptUpdateError::SquashBody)?;
                    };
                }
                MixedCheque::Cheque(cheque) => {
                    if proposed.is_index_squashed(cheque.cheque_body.index) {
                        return Err(MixedReceiptUpdateError::IncludesCheque);
                    }
                }
            };
        }
        if current == *proposed {
            self.squash = squash;
            self.mixed_cheques
                .retain(|mc| !self.squash.squash_body.is_index_squashed(mc.index()));
            Ok(self.unlockeds().is_empty())
        } else {
            Err(MixedReceiptUpdateError::NotReproduced)
        }
    }

    pub fn make_squash_body(&self) -> Result<SquashBody, MixedReceiptUpdateError> {
        let mut squash_body = self.squash.squash_body.clone();
        for mixed_cheque in &self.mixed_cheques {
            if let MixedCheque::Unlocked(unlocked) = mixed_cheque {
                squash_body
                    .squash(unlocked.cheque_body.clone())
                    .map_err(MixedReceiptUpdateError::SquashBody)?;
            };
        }
        Ok(squash_body)
    }
}

impl<'a> TryFrom<Vec<PlutusData<'a>>> for MixedReceipt {
    type Error = anyhow::Error;

    fn try_from(value: Vec<PlutusData<'a>>) -> anyhow::Result<Self> {
        Self::try_from(<[PlutusData; 2]>::try_from(value).map_err(|_| anyhow!("Bad length"))?)
    }
}

impl<'a> TryFrom<[PlutusData<'a>; 2]> for MixedReceipt {
    type Error = anyhow::Error;

    fn try_from(value: [PlutusData<'a>; 2]) -> anyhow::Result<Self> {
        let [a, b] = value;
        Self::new(
            Squash::try_from(&a)?,
            <Vec<PlutusData>>::try_from(&b)?
                .into_iter()
                .map(MixedCheque::try_from)
                .collect::<anyhow::Result<Vec<MixedCheque>>>()?,
        )
    }
}

impl<'a> TryFrom<&PlutusData<'a>> for MixedReceipt {
    type Error = anyhow::Error;

    fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
        Self::try_from(<[PlutusData; 2]>::try_from(data)?)
    }
}

impl<'a> From<MixedReceipt> for [PlutusData<'a>; 2] {
    fn from(value: MixedReceipt) -> Self {
        [
            PlutusData::from(value.squash),
            PlutusData::list(
                value
                    .mixed_cheques
                    .into_iter()
                    .map(PlutusData::from)
                    .collect::<Vec<PlutusData>>(),
            ),
        ]
    }
}

impl<'a> From<MixedReceipt> for PlutusData<'a> {
    fn from(value: MixedReceipt) -> Self {
        Self::list(<[PlutusData; 2]>::from(value).to_vec())
    }
}
