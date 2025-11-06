use crate::{Duration, Tag};
use anyhow::anyhow;
use cardano_tx_builder::{PlutusData, VerificationKey, constr};

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct Constants {
    pub tag: Tag,
    pub add_vkey: VerificationKey,
    pub sub_vkey: VerificationKey,
    pub close_period: Duration,
}

impl Constants {
    pub fn verify(&self, max_tag_length: usize, min_close_period: u64) -> bool {
        self.tag.0.len() <= max_tag_length
            && self.close_period.as_millis() >= min_close_period as u128
    }
}

impl<'a> TryFrom<&PlutusData<'a>> for Constants {
    type Error = anyhow::Error;

    fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
        let (tag, fields) = data.as_constr().ok_or(anyhow!("Not a constructor"))?;
        if tag != 0 {
            return Err(anyhow!("Bad constructor tag"));
        }
        let [a, b, c, d] = <[PlutusData; 4]>::try_from(fields.collect::<Vec<_>>())
            .map_err(|_| anyhow!("invalid 'Cheque'"))?;
        Ok(Self {
            tag: Tag::try_from(&a)?,
            add_vkey: VerificationKey::from(*<&[u8; 32]>::try_from(&b)?),
            sub_vkey: VerificationKey::from(*<&[u8; 32]>::try_from(&c)?),
            close_period: Duration::try_from(&d)?,
        })
    }
}

impl<'a> From<Constants> for PlutusData<'a> {
    fn from(value: Constants) -> Self {
        constr!(
            0,
            PlutusData::from(value.tag),
            PlutusData::from(<[u8; 32]>::from(value.add_vkey)),
            PlutusData::from(<[u8; 32]>::from(value.sub_vkey)),
            PlutusData::from(value.close_period),
        )
    }
}
