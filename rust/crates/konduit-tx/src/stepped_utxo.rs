use cardano_sdk::{Output, PlutusData, VerificationKey};
use konduit_data::Step;

use crate::{
    Bounds, ChannelUtxo, Stepped,
    error_and::ErrorAnd,
    step_and::StepAnd,
    stepping::{Error, Stepping},
    utxo_and::UtxoAnd,
};

pub type SteppedUtxo = UtxoAnd<Stepped>;

/// Go "back" to a `ChannelUtxo`
impl From<SteppedUtxo> for ChannelUtxo {
    fn from(value: SteppedUtxo) -> Self {
        Self::new(value.utxo().to_owned(), value.data().data().to_owned())
    }
}

impl SteppedUtxo {
    pub fn stepping(&self) -> &Stepping {
        self.data().stepping()
    }

    pub fn step(&self) -> Step {
        self.data().stepping().step()
    }

    pub fn cont_output(&self) -> Option<Output> {
        self.data().cont_data().map(|channel_data| {
            Output::new(
                self.output().address().clone(),
                channel_data.buffered_value(),
            )
            .with_datum(PlutusData::from(channel_data.datum()))
        })
    }

    pub fn bounds(&self) -> Bounds {
        self.data().stepping().bounds()
    }

    fn consumer_key(&self) -> VerificationKey {
        self.data().data().constants().add_vkey.clone()
    }

    fn adaptor_key(&self) -> VerificationKey {
        self.data().data().constants().sub_vkey.clone()
    }

    pub fn signer(&self) -> VerificationKey {
        if self.step().is_consumer() {
            self.consumer_key()
        } else {
            self.adaptor_key()
        }
    }

    pub fn gain(&self) -> i64 {
        self.data().data().buffered_amount() as i64
            - self.data().cont_data().map_or(0, |x| x.buffered_amount()) as i64
    }
}

/// With the context available, create the most sensible "stepping".
/// Gemini assures me this "kinda idiomatic"
impl TryFrom<(ChannelUtxo, StepAnd)> for SteppedUtxo {
    type Error = ErrorAnd<ChannelUtxo, Error>;

    fn try_from(value: (ChannelUtxo, StepAnd)) -> Result<Self, Self::Error> {
        let (channel_utxo, step_and) = value;
        match Stepping::new(channel_utxo.variables(), step_and) {
            Ok(stepping) => Ok(Self::new(
                channel_utxo.utxo().to_owned(),
                Stepped::new(channel_utxo.data().to_owned(), stepping),
            )),
            Err(err) => Err(ErrorAnd(channel_utxo, err)),
        }
    }
}
