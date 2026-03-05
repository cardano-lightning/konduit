use konduit_data::{Constants, Datum, PossibleStep, Stage};

use crate::{
    KONDUIT_VALIDATOR, Utxo,
    channel_data::{self, ChannelData},
    channel_variables::Variables,
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

pub type ChannelUtxo = UtxoAnd<ChannelData>;

impl TryFrom<Utxo> for ChannelUtxo {
    type Error = channel_data::Error;

    fn try_from(utxo: Utxo) -> Result<Self, Self::Error> {
        let data = ChannelData::try_from(&utxo.1)?;
        Ok(Self::new(utxo, data))
    }
}

impl ChannelUtxo {
    pub fn possible_steps(&self) -> Vec<PossibleStep> {
        self.data().stage().possible_steps()
    }

    pub fn variables(&self) -> Variables {
        Variables::from(self.data())
    }
}
