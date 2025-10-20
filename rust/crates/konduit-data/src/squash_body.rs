use std::cmp;

use anyhow::anyhow;
use cardano_tx_builder::PlutusData;
use cardano_tx_builder::cbor::ToCbor;

use crate::MAX_EXCLUDE_LENGTH;
use crate::cheque_body::ChequeBody;

#[derive(Debug, Clone)]
pub struct SquashBody {
    pub amount: u64,
    pub index: u64,
    pub exclude: Vec<u64>,
}

impl SquashBody {
    pub fn verify_new(index: u64, exclude: &Vec<u64>) -> anyhow::Result<()> {
        if !exclude.windows(2).all(|w| w[0] < w[1]) {
            Err(anyhow!("Exclude must be strictly monotonically increasing"))?;
        }
        if let Some(last) = exclude.last()
            && last > &index
        {
            Err(anyhow!("Exclude must be strictly less than index"))?;
        }
        if exclude.len() > MAX_EXCLUDE_LENGTH {
            Err(anyhow!("Exclude exceeds max length"))?;
        }
        Ok(())
    }

    pub fn new(amount: u64, index: u64, exclude: Vec<u64>) -> anyhow::Result<Self> {
        SquashBody::verify_new(index, &exclude)?;
        SquashBody::new_no_verify(amount, index, exclude)
    }

    pub fn new_no_verify(amount: u64, index: u64, exclude: Vec<u64>) -> anyhow::Result<Self> {
        Ok(Self {
            amount,
            index,
            exclude,
        })
    }

    pub fn tagged_bytes(&self, tag: Vec<u8>) -> Vec<u8> {
        let mut tag = tag.clone();
        let mut bytes = PlutusData::from(self).to_cbor();
        tag.append(&mut bytes);
        tag
    }

    /// Only squash what has been verified
    pub fn squash(&self, cheque_body: ChequeBody) -> anyhow::Result<Self> {
        match self.index.cmp(&cheque_body.index) {
            cmp::Ordering::Less => {
                let mut exclude = self.exclude.clone();
                let mut append: Vec<u64> = ((self.index + 1)..cheque_body.index).collect();
                exclude.append(&mut append);
                Self::new(cheque_body.index, self.amount + cheque_body.amount, exclude)
            }
            cmp::Ordering::Equal => Err(anyhow!("Cannot squash when indexes are equal")),
            cmp::Ordering::Greater => {
                match self.exclude.iter().position(|x| x == &cheque_body.index) {
                    Some(position) => {
                        let mut exclude = self.exclude.clone();
                        exclude.remove(position);
                        Self::new(self.index, self.amount + cheque_body.amount, exclude)
                    }
                    None => Err(anyhow!("Index already squashed")),
                }
            }
        }
    }

    pub fn index_squashed(&self, index: u64) -> bool {
        match self.index.cmp(&index) {
            cmp::Ordering::Less => false,
            cmp::Ordering::Equal => true,
            cmp::Ordering::Greater => self.exclude.iter().all(|x| x != &index),
        }
    }
}

impl<'a> TryFrom<PlutusData<'a>> for SquashBody {
    type Error = anyhow::Error;

    fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
        let [a, b, c] = <[PlutusData; 3]>::try_from(&data)?;
        let exclude = <Vec<PlutusData>>::try_from(&c)?
            .iter()
            .map(|x| <u64>::try_from(x))
            .collect::<anyhow::Result<Vec<u64>>>()?;
        Ok(Self::new(
            <u64>::try_from(&a)?,
            <u64>::try_from(&b)?,
            exclude,
        )?)
    }
}

impl<'a> From<&SquashBody> for PlutusData<'a> {
    fn from(value: &SquashBody) -> Self {
        Self::list(vec![
            PlutusData::from(value.amount),
            PlutusData::from(value.index),
            PlutusData::list(
                value
                    .exclude
                    .iter()
                    .map(|x| PlutusData::from(x.clone()))
                    .collect::<Vec<PlutusData>>(),
            ),
        ])
    }
}
