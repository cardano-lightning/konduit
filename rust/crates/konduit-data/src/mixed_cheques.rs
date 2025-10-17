use anyhow::{Error, Result, anyhow};
use cardano_tx_builder::PlutusData;

use crate::{
    base::{Amount, Timestamp},
    cheque::Cheque,
    mixed_cheque::MixedCheque,
    pending::Pending,
    pendings::Pendings,
};

#[derive(Debug, Clone)]
pub struct MixedCheques(pub Vec<MixedCheque>);

impl MixedCheques {
    pub fn max_timeout(&self) -> Option<Timestamp> {
        // TODO::We don't need unlockeds. We need an upper_bound
        self.0
            .iter()
            .filter_map(|x| match x {
                MixedCheque::Unlocked(unlocked) => Some(unlocked.cheque_body.timeout.0),
                _ => None,
            })
            .max()
            .map(Timestamp)
    }

    pub fn amount(&self) -> Amount {
        Amount(
            self.0
                .iter()
                .map(|x| match x {
                    MixedCheque::Unlocked(unlocked) => unlocked.cheque_body.amount.0,
                    _ => 0,
                })
                .sum(),
        )
    }

    pub fn pendings(&self) -> (Amount, Pendings) {
        let pendings: Vec<Pending> = self
            .0
            .iter()
            .filter_map(|x| match x {
                MixedCheque::Unlocked(_) => None,
                MixedCheque::Cheque(Cheque {
                    cheque_body: body, ..
                }) => Some(Pending::new(
                    body.amount.clone(),
                    body.timeout.clone(),
                    body.lock.clone(),
                )),
            })
            .collect();
        let amount = pendings.iter().map(|x| x.amount.0).sum();
        (Amount(amount), Pendings(pendings))
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
