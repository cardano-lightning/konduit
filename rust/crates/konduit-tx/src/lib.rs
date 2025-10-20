use cardano_tx_builder::{Input, Output};
use std::collections::BTreeMap;

pub mod can_step;
pub mod channel;
pub mod constraints;
pub mod intent;

mod txs;
pub use txs::*;

pub type Utxos = BTreeMap<Input, Output>;
