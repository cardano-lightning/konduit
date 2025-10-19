use std::collections::BTreeMap;

use cardano_tx_builder::{Input, Output};

pub mod can_step;
pub mod channel;
pub mod constraints;
pub mod intent;
pub mod txs;

pub type Utxos = BTreeMap<Input, Output>;
