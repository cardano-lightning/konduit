use anyhow::anyhow;
use cardano_tx_builder::PlutusData;

use crate::{MAX_UNSQUASHED, Squash, Unlocked};

#[derive(Debug, Clone)]
pub struct Receipt {
    pub squash: Squash,
    pub unlockeds: Vec<Unlocked>,
}

impl Receipt {
    pub fn new(squash: Squash, unlockeds: Vec<Unlocked>) -> anyhow::Result<Self> {
        if unlockeds.len() > MAX_UNSQUASHED {
            Err(anyhow!("Too many unlockeds"))?;
        }
        let mut sorted: Vec<Unlocked> = vec![];
        for unlocked in unlockeds {
            let index = unlocked.cheque_body.index;
            if squash.squash_body.index_squashed(index) {
                Err(anyhow!("Index {} is already squashed", index))?;
            }
            match sorted.binary_search_by(|probe| probe.cheque_body.index.cmp(&index)) {
                Ok(_) => Err(anyhow!("Duplicate index {}", index))?,
                Err(position) => sorted.insert(position, unlocked.clone()),
            };
        }
        sorted.sort();
        Ok(Self::new_no_verify(squash, sorted))
    }

    pub fn new_no_verify(squash: Squash, unlockeds: Vec<Unlocked>) -> Self {
        Self { squash, unlockeds }
    }

    pub fn amount(&self) -> u64 {
        self.squash.amount()
            + self
                .unlockeds
                .iter()
                .map(|x| x.cheque_body.amount)
                .sum::<u64>()
    }
}

impl<'a> TryFrom<Vec<PlutusData<'a>>> for Receipt {
    type Error = anyhow::Error;

    fn try_from(value: Vec<PlutusData<'a>>) -> anyhow::Result<Self> {
        Self::try_from(<[PlutusData; 2]>::try_from(value).map_err(|_| anyhow!("Bad length"))?)
    }
}

impl<'a> TryFrom<[PlutusData<'a>; 2]> for Receipt {
    type Error = anyhow::Error;

    fn try_from(value: [PlutusData<'a>; 2]) -> anyhow::Result<Self> {
        let [a, b] = value;
        Ok(Self::new(
            Squash::try_from(&a)?,
            <Vec<PlutusData>>::try_from(&b)?
                .iter()
                .map(|x| Unlocked::try_from(x))
                .collect::<anyhow::Result<Vec<Unlocked>>>()?,
        )?)
    }
}

impl<'a> TryFrom<&PlutusData<'a>> for Receipt {
    type Error = anyhow::Error;

    fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
        Self::try_from(<[PlutusData; 2]>::try_from(data)?)
    }
}

impl<'a> From<Receipt> for [PlutusData<'a>; 2] {
    fn from(value: Receipt) -> Self {
        [
            PlutusData::from(value.squash),
            PlutusData::list(
                value
                    .unlockeds
                    .into_iter()
                    .map(|x| PlutusData::from(x))
                    .collect::<Vec<PlutusData>>(),
            ),
        ]
    }
}

impl<'a> From<Receipt> for PlutusData<'a> {
    fn from(value: Receipt) -> Self {
        Self::list(<[PlutusData; 2]>::from(value).to_vec())
    }
}
