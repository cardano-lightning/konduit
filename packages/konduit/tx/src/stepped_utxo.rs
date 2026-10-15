use std::cmp;

use cardano_sdk::{Output, PlutusData, VerificationKey};
use konduit_data::{AssetId, Step};

use crate::{Bounds, ChannelUtxo, MIN_ADA_BUFFER, Stepped, from_verifying_key, utxo_and::UtxoAnd};

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
            let address = self.output().address().clone();
            let value = channel_data.buffered_value();
            let datum = PlutusData::from(channel_data.datum());
            if channel_data.constants().asset == AssetId::Ada {
                return Output::new(address, value).with_datum(datum);
            }
            let min_lovelace = Output::new(address.clone(), value.clone())
                .with_datum(datum.clone())
                .min_acceptable_value();
            let mut value = value;
            value.with_lovelace(cmp::max(
                self.output().value().lovelace(),
                cmp::max(MIN_ADA_BUFFER, min_lovelace),
            ));
            Output::new(address, value).with_datum(datum)
        })
    }

    pub fn bounds(&self) -> &Bounds {
        self.data().bounds()
    }

    fn consumer_key(&self) -> VerificationKey {
        from_verifying_key(self.data().channel().constants().add_vkey)
    }

    fn adaptor_key(&self) -> VerificationKey {
        from_verifying_key(self.data().channel().constants().sub_vkey)
    }

    pub fn signer(&self) -> VerificationKey {
        if self.step().is_consumer() {
            self.consumer_key()
        } else {
            self.adaptor_key()
        }
    }

    pub fn gain(&self) -> i64 {
        let gain = self.gain_i128();
        i64::try_from(gain).unwrap_or(if gain.is_negative() {
            i64::MIN
        } else {
            i64::MAX
        })
    }

    pub(crate) fn gain_i128(&self) -> i128 {
        i128::from(self.data().channel().amount())
            - i128::from(self.data().cont_data().map_or(0, |x| x.amount()))
    }
}
