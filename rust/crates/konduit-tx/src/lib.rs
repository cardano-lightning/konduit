use std::collections::BTreeMap;

use cardano_tx_builder::{Input, Output};

mod can_step;
mod channel;
mod constraints;
mod intent;
mod txs;

pub type Utxos = BTreeMap<Input, Output>;
