use anyhow::anyhow;
use cardano_tx_builder::PlutusData;

use crate::{Cheque, MAX_UNSQUASHED, MixedCheque, Squash, Unlocked};

#[derive(Debug, Clone)]
pub struct MixedReceipt {
    pub squash: Squash,
    pub mixed_cheques: Vec<MixedCheque>,
}

impl MixedReceipt {
    pub fn new(squash: Squash, mixed_cheques: Vec<MixedCheque>) -> anyhow::Result<Self> {
        if mixed_cheques.len() > MAX_UNSQUASHED {
            Err(anyhow!("Too many unsquashed"))?;
        }
        let mut sorted: Vec<MixedCheque> = vec![];
        for mixed_cheque in mixed_cheques {
            let index = mixed_cheque.index();
            if squash.squash_body.index_squashed(index) {
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

    pub fn amount(&self) -> u64 {
        self.squash.amount()
            + self
                .unlockeds()
                .iter()
                .map(|x| x.cheque_body.amount)
                .sum::<u64>()
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
        Ok(Self::new(
            Squash::try_from(&a)?,
            <Vec<PlutusData>>::try_from(&b)?
                .into_iter()
                .map(|x| MixedCheque::try_from(x))
                .collect::<anyhow::Result<Vec<MixedCheque>>>()?,
        )?)
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
                    .map(|x| PlutusData::from(x))
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
