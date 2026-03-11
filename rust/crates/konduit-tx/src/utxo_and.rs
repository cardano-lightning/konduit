use cardano_sdk::{Input, Output};

use crate::Utxo;

/// We inspect a Utxo once, and then parse it around, endowed with extra data.
#[derive(Debug, Clone)]
pub struct UtxoAnd<T> {
    utxo: Utxo,
    data: T,
}

impl<T> UtxoAnd<T> {
    pub fn new(utxo: Utxo, data: T) -> Self {
        Self { utxo, data }
    }

    pub fn utxo(&self) -> &Utxo {
        &self.utxo
    }

    pub fn input(&self) -> &Input {
        &self.utxo.0
    }

    pub fn output(&self) -> &Output {
        &self.utxo.1
    }

    pub fn data(&self) -> &T {
        &self.data
    }
}
