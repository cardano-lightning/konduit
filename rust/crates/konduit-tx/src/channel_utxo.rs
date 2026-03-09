use konduit_data::{Constants, Datum, PossibleStep, Stage};

use crate::{
    KONDUIT_VALIDATOR, Utxo,
    channel::{self, Channel},
    utxo_and::UtxoAnd,
};

fn konduit_datum(constants: Constants, stage: Stage) -> Datum {
    Datum {
        own_hash: KONDUIT_VALIDATOR.hash,
        constants,
        stage,
    }
}

// Process:
// - Find channels.
// - Filter channels.
// - Source steps.
// - Select steps.
// - Build tx.

pub type ChannelUtxo = UtxoAnd<Channel>;

impl TryFrom<Utxo> for ChannelUtxo {
    type Error = channel::Error;

    fn try_from(utxo: Utxo) -> Result<Self, Self::Error> {
        let data = Channel::try_from(&utxo.1)?;
        Ok(Self::new(utxo, data))
    }
}

impl ChannelUtxo {
    pub fn possible_steps(&self) -> Vec<PossibleStep> {
        self.data().stage().possible_steps()
    }
}
