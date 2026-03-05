use std::collections::BTreeMap;

use cardano_sdk::{Hash, Input, Output, VerificationKey};
use konduit_data::{Redeemer, Step};

use crate::{Bounds, SteppedUtxo};

#[derive(Debug, Clone)]
pub struct SteppedUtxos(Vec<SteppedUtxo>);

impl From<Vec<SteppedUtxo>> for SteppedUtxos {
    fn from(value: Vec<SteppedUtxo>) -> Self {
        let mut value = value;
        value.sort_by_key(|x| x.input().clone());
        Self(value)
    }
}

impl SteppedUtxos {
    pub fn inputs(&self) -> Vec<(Input, Redeemer)> {
        self.0
            .iter()
            .enumerate()
            .map(|(i, val)| (val.input().clone(), self.redeemer(i)))
            .collect::<Vec<_>>()
    }

    fn redeemer(&self, index: usize) -> Redeemer {
        if index == 0 {
            Redeemer::Main(self.steps())
        } else {
            Redeemer::Defer
        }
    }

    pub fn steps(&self) -> Vec<Step> {
        self.0
            .iter()
            .map(|x| x.data().stepping().step())
            .collect::<Vec<_>>()
    }

    pub fn utxos(&self) -> BTreeMap<Input, Output> {
        self.0
            .iter()
            .map(|x| x.utxo())
            .cloned()
            .collect::<BTreeMap<_, _>>()
    }

    /// Continuing outputs
    pub fn outputs(&self) -> Vec<Output> {
        self.0
            .iter()
            .filter_map(|x| x.cont_output())
            .collect::<Vec<_>>()
    }

    pub fn signers(&self) -> Vec<VerificationKey> {
        let mut signers = self.0.iter().map(|x| x.signer()).collect::<Vec<_>>();
        signers.sort();
        signers.dedup();
        signers
    }

    pub fn specified_signatories(&self) -> Vec<Hash<28>> {
        self.signers()
            .iter()
            .map(|x| Hash::<28>::new(x))
            .collect::<Vec<_>>()
    }

    pub fn bounds(&self) -> Bounds {
        self.0.iter().fold(Bounds::default(), |bounds, item| {
            bounds.intersect(&item.bounds())
        })
    }

    pub fn gain(&self) -> i64 {
        self.0.iter().map(|x| x.gain()).sum::<i64>()
    }
}
