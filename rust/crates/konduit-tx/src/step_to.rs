use konduit_data::{Cont, Eol, Step};

use crate::Variables;

#[derive(Debug, Clone)]
pub enum StepTo {
    Cont(Cont, Box<Variables>),
    Eol(Eol),
}

impl StepTo {
    pub fn cont(step: Cont, variables: Variables) -> Self {
        Self::Cont(step, Box::new(variables))
    }

    pub fn eol(step: Eol) -> Self {
        Self::Eol(step)
    }

    pub fn step(&self) -> Step {
        match &self {
            StepTo::Cont(cont, _) => Step::Cont(cont.clone()),
            StepTo::Eol(eol) => Step::Eol(eol.clone()),
        }
    }

    pub fn variables(&self) -> Option<Variables> {
        match &self {
            Self::Cont(_, o) => Some(o.as_ref().clone()),
            Self::Eol(_) => None,
        }
    }
}
