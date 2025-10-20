use cardano_tx_builder::VerificationKey;
use konduit_data::Timestamp;
use std::cmp::{max, min};

#[derive(Debug, Clone, Default)]
pub struct Constraints {
    pub lower_bound: Option<Timestamp>,
    pub upper_bound: Option<Timestamp>,
    pub required_signers: Vec<VerificationKey>,
}

impl Constraints {
    pub fn new(
        lower_bound: Option<Timestamp>,
        upper_bound: Option<Timestamp>,
        required_signers: Vec<VerificationKey>,
    ) -> Self {
        Self {
            lower_bound,
            upper_bound,
            required_signers,
        }
    }

    pub fn new_required_signer(vkey: VerificationKey) -> Self {
        Self::new_required_signers(vec![vkey])
    }

    pub fn new_required_signers(vkeys: Vec<VerificationKey>) -> Self {
        Self::new(None, None, vkeys)
    }

    pub fn with_lower_bound(mut self, lower_bound: Timestamp) -> Self {
        let lower_bound = match self.lower_bound {
            None => lower_bound,
            Some(current) => Timestamp(max(current.0, lower_bound.0)),
        };
        self.lower_bound = Some(lower_bound);
        self
    }

    pub fn with_upper_bound(mut self, upper_bound: Timestamp) -> Self {
        let upper_bound = match self.upper_bound {
            None => upper_bound.clone(),
            Some(current) => Timestamp(min(current.0, upper_bound.clone().0)),
        };
        self.upper_bound = Some(upper_bound);
        self
    }

    pub fn with_required_signer(mut self, vkey: &VerificationKey) -> Self {
        if !self.required_signers.contains(vkey) {
            self.required_signers.push(vkey.clone())
        }
        self
    }

    pub fn merge(mut self, other: Self) -> Self {
        if let Some(lower_bound) = other.lower_bound {
            self = self.with_lower_bound(lower_bound)
        }
        if let Some(upper_bound) = other.upper_bound {
            self = self.with_upper_bound(upper_bound)
        }
        for vkey in other.required_signers {
            self = self.with_required_signer(&vkey)
        }
        self
    }
}
