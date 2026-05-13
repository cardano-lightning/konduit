use std::collections::BTreeMap;

use cardano_sdk::{Input, Output};

pub type Utxos = BTreeMap<Input, Output>;
