use crate::{Cheque, Lock, MixedCheque, Pending, Pendings, Timestamp};
use anyhow::{Error, Result, anyhow};
use cardano_tx_builder::PlutusData;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct MixedCheques(pub Vec<MixedCheque>);

impl MixedCheques {
    pub fn max_timeout(&self) -> Option<Timestamp> {
        // TODO::We don't need unlockeds. We need an upper_bound
        self.0
            .iter()
            .filter_map(|x| match x {
                MixedCheque::Unlocked(unlocked) => Some(unlocked.cheque_body.timeout),
                _ => None,
            })
            .max()
            .map(|t| Timestamp(Duration::from_millis(t)))
    }

    pub fn amount(&self) -> u64 {
        self.0
            .iter()
            .map(|x| match x {
                MixedCheque::Unlocked(unlocked) => unlocked.cheque_body.amount,
                _ => 0,
            })
            .sum()
    }

    pub fn pendings(&self) -> (u64, Pendings) {
        let pendings: Vec<Pending> = self
            .0
            .iter()
            .filter_map(|x| match x {
                MixedCheque::Unlocked(_) => None,
                MixedCheque::Cheque(Cheque {
                    cheque_body: body, ..
                }) => Some(Pending::new(
                    body.amount,
                    Timestamp(Duration::from_millis(body.timeout)),
                    Lock(body.lock),
                )),
            })
            .collect();
        let amount = pendings.iter().map(|x| x.amount).sum();
        (amount, Pendings(pendings))
    }
}

impl<'a> TryFrom<&PlutusData<'a>> for MixedCheques {
    type Error = Error;

    fn try_from(data: &PlutusData<'a>) -> Result<Self> {
        let list = data.as_list().ok_or(anyhow!("Expect list"))?;
        let inner = list
            .into_iter()
            .map(MixedCheque::try_from)
            .collect::<Result<Vec<MixedCheque>>>()?;
        Ok(MixedCheques(inner))
    }
}

impl<'a> From<MixedCheques> for PlutusData<'a> {
    fn from(value: MixedCheques) -> Self {
        Self::list(value.0)
    }
}
