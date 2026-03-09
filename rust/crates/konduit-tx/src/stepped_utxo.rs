use cardano_sdk::{Output, PlutusData, VerificationKey};
use konduit_data::Step;

use crate::{Bounds, ChannelUtxo, Stepped, utxo_and::UtxoAnd};

pub type SteppedUtxo = UtxoAnd<Stepped>;

/// Go "back" to a `ChannelUtxo`
impl From<SteppedUtxo> for ChannelUtxo {
    fn from(value: SteppedUtxo) -> Self {
        Self::new(value.utxo().to_owned(), value.data().channel().to_owned())
    }
}

impl SteppedUtxo {
    pub fn step(&self) -> Step {
        self.data().step_to().step()
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

    pub fn bounds(&self) -> &Bounds {
        self.data().bounds()
    }

    fn consumer_key(&self) -> VerificationKey {
        self.data().channel().constants().add_vkey.clone()
    }

    fn adaptor_key(&self) -> VerificationKey {
        self.data().channel().constants().sub_vkey.clone()
    }

    pub fn signer(&self) -> VerificationKey {
        if self.step().is_consumer() {
            self.consumer_key()
        } else {
            self.adaptor_key()
        }
    }

    pub fn gain(&self) -> i64 {
        self.data().channel().buffered_amount() as i64
            - self.data().cont_data().map_or(0, |x| x.buffered_amount()) as i64
    }
}
