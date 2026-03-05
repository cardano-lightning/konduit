use cardano_sdk::{Input, Output, PlutusData, VerificationKey};
use konduit_data::{Constants, Duration, Stage, Step};

use crate::{Bounds, Utxo, Utxos, konduit_utxo::Channel};

#[derive(Debug, Clone)]
pub struct OpenIntent {
    pub constants: Constants,
    pub amount: u64,
}

#[derive(Debug, Clone)]
pub enum Intent {
    Add(u64),
    Close,
}

pub struct ConsumerContext<T> {
    channels: Vec<UtxoAnd<T>>,
    opens: Vec<OpenIntent>,
    intents: Vec<Intent>,
    keys: Vec<VerificationKey>,
    bounds: Bounds,
    channels: Channel,
    fuel: Utxos,
}

pub struct ConsumerChannel {
    utxo: Utxo,
    channel: Channel,
}
