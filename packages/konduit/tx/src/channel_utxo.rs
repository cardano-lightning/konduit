use cardano_sdk::{Input, Output};
use konduit_data::{Cheque, Duration, Secret, Squash, Unlocked, Verified};

use crate::{
    StepError, SteppedUtxo, Utxo,
    channel::{self, Channel, SteppedElseChannel},
    utxo_and::UtxoAnd,
};

// Process:
// - Find channels.
// - Filter channels.
// - Source steps.
// - Select steps.
// - Build tx.

pub type ChannelUtxo = UtxoAnd<Channel>;

impl TryFrom<(&Input, &Output)> for ChannelUtxo {
    type Error = channel::Error;

    fn try_from((input, output): (&Input, &Output)) -> Result<Self, Self::Error> {
        let data = Channel::try_from(output)?;
        Ok(Self::new((input.clone(), output.clone()), data))
    }
}

impl TryFrom<Utxo> for ChannelUtxo {
    type Error = channel::Error;

    fn try_from(utxo: Utxo) -> Result<Self, Self::Error> {
        let data = Channel::try_from(&utxo.1)?;
        Ok(Self::new(utxo, data))
    }
}

type SteppedElseChannelUtxo = Result<SteppedUtxo, (Box<ChannelUtxo>, StepError)>;

impl ChannelUtxo {
    fn rewrap(utxo: Utxo, result: SteppedElseChannel) -> SteppedElseChannelUtxo {
        match result {
            Ok(data) => Ok(UtxoAnd::new(utxo, data)),
            Err((data, err)) => Err((Box::new(UtxoAnd::new(utxo, *data)), err)),
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn add(self, amount: u64) -> SteppedElseChannelUtxo {
        Self::rewrap(self.utxo().to_owned(), self.data().to_owned().add(amount))
    }

    pub fn sub(
        self,
        squash: Squash<Verified>,
        unlockeds: Vec<Unlocked<Verified>>,
        upper: &Duration,
    ) -> SteppedElseChannelUtxo {
        Self::rewrap(
            self.utxo().to_owned(),
            self.data().to_owned().sub(squash, unlockeds, upper),
        )
    }

    pub fn close(self, upper: &Duration) -> SteppedElseChannelUtxo {
        Self::rewrap(self.utxo().to_owned(), self.data().to_owned().close(upper))
    }

    pub fn elapse(self, lower: &Duration) -> SteppedElseChannelUtxo {
        Self::rewrap(self.utxo().to_owned(), self.data().to_owned().elapse(lower))
    }

    pub fn respond(
        self,
        squash: Squash<Verified>,
        cheques: Vec<Cheque<Verified>>,
        upper: &Duration,
    ) -> SteppedElseChannelUtxo {
        Self::rewrap(
            self.utxo().to_owned(),
            self.data().to_owned().respond(squash, cheques, upper),
        )
    }

    pub fn unlock(self, secrets: Vec<Secret>, upper: &Duration) -> SteppedElseChannelUtxo {
        Self::rewrap(
            self.utxo().to_owned(),
            self.data().to_owned().unlock(secrets, upper),
        )
    }

    pub fn expire(self, lower: &Duration) -> SteppedElseChannelUtxo {
        Self::rewrap(self.utxo().to_owned(), self.data().to_owned().expire(lower))
    }

    pub fn end(self, lower: Option<&Duration>) -> SteppedElseChannelUtxo {
        Self::rewrap(self.utxo().to_owned(), self.data().to_owned().end(lower))
    }
}
