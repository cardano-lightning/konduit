use anyhow::{Result, anyhow};

use pallas_primitives::PlutusData;

use super::base::{Amount, Hash32, Index, Timestamp};
use super::plutus::{self, PData};

#[derive(Debug, Clone)]
pub struct ChequeBody {
    index: Index,
    amount: Amount,
    timeout: Timestamp,
    image: Hash32,
}

impl ChequeBody {
    pub fn new(index: Index, amount: Amount, timeout: Timestamp, image: Hash32) -> Self {
        Self {
            index,
            amount,
            timeout,
            image,
        }
    }
}

impl PData for ChequeBody {
    fn to_plutus_data(self: &Self) -> PlutusData {
        let data = plutus::list(&vec![
            self.index.to_plutus_data(),
            self.amount.to_plutus_data(),
            self.timeout.to_plutus_data(),
            self.image.to_plutus_data(),
        ]);
        data
    }

    fn from_plutus_data(d: &PlutusData) -> Result<ChequeBody> {
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
