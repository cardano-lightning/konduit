use super::base::{Tag, TimeDelta, VKey};
use anyhow::anyhow;

use pallas_primitives::PlutusData;

use super::plutus::{self, PData};

#[derive(Debug, Clone)]
pub struct Constants {
    pub tag: Tag,
    pub add_vkey: VKey,
    pub sub_vkey: VKey,
    pub close_period: TimeDelta,
}

impl Constants {
    pub fn new(tag: Tag, add_vkey: VKey, sub_vkey: VKey, close_period: TimeDelta) -> Self {
        Self {
            tag,
            add_vkey,
            sub_vkey,
            close_period,
        }
    }
}

impl PData for Constants {
    fn to_plutus_data(self: &Self) -> PlutusData {
        let data = plutus::list(&vec![
            self.tag.to_plutus_data(),
            self.add_vkey.to_plutus_data(),
            self.sub_vkey.to_plutus_data(),
            self.close_period.to_plutus_data(),
        ]);
        data
    }

    fn from_plutus_data(d: &PlutusData) -> anyhow::Result<Constants> {
        match &plutus::unlist(d)?[..] {
            [a, b, c, d] => {
                let r = Self::new(
                    PData::from_plutus_data(a)?,
                    PData::from_plutus_data(b)?,
                    PData::from_plutus_data(c)?,
                    PData::from_plutus_data(d)?,
                );
                Ok(r)
            }
            _ => Err(anyhow!("bad length")),
        }
    }
}
