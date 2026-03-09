use std::{cmp, collections::BTreeMap};

use konduit_data::{
    Cont, Duration, Eol, Lock, Pending, Receipt, Secret, Stage, Step, Unpend, Used,
};

use crate::{Bounds, StepTo, step_and::StepAnd, variables::Variables};

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("Step had no effect on variables")]
    NoStep,
    #[error("Too early: Set {0}, Need > {0}")]
    Early(Duration, Duration),
    #[error("Pair (Stage, Step) ({0}, {1}) are incompat")]
    Pair(String, String),
    #[error("Terminal `Expire`. Should be `End`")]
    Expire,
    #[error("Other :: {0}")]
    Other(String),
}

/// Encapsulate the step and where applicable then the continuing output variables.
#[derive(Debug, Clone)]
pub struct Stepping {
    bounds: Bounds,
    step_to: StepTo,
}

impl Stepping {
    pub fn new(variables: Variables, step_and: StepAnd) -> Result<Self, Error> {
        let amount = variables.amount();
        let stage = variables.stage();
        let stepping = match (&stage, &step_and) {
            (Stage::Opened(subbed, useds), StepAnd::Add { amount: add }) => {
                Self::add(amount, subbed.to_owned(), useds.to_owned(), add.to_owned())
            }
            (Stage::Opened(subbed, useds), StepAnd::Sub { receipt, upper }) => Self::sub(
                amount,
                subbed.to_owned(),
                useds.to_owned(),
                receipt.to_owned(),
                upper.to_owned(),
            ),
            (
                Stage::Opened(subbed, useds),
                StepAnd::Close {
                    upper,
                    close_period,
                },
            ) => Self::close(
                amount,
                subbed.to_owned(),
                useds.to_owned(),
                upper.to_owned(),
                close_period.to_owned(),
            ),
            (Stage::Closed(subbed, useds, _), StepAnd::Respond { receipt, upper }) => {
                Self::respond(
                    amount,
                    subbed.to_owned(),
                    useds.to_owned(),
                    receipt.to_owned(),
                    upper.to_owned(),
                )
            }
            (Stage::Closed(_, _, elapse_at), StepAnd::Elapse { lower }) => {
                Self::elapse(elapse_at.to_owned(), lower.to_owned())?
            }
            (Stage::Responded(_, pendings), StepAnd::Expire { lower }) => {
                Self::expire(pendings.to_owned(), lower.to_owned())
            }
            (Stage::Responded(_, pendings), StepAnd::Unlock { receipt, upper }) => Self::unlock(
                amount,
                pendings.to_owned(),
                receipt.secrets(),
                upper.to_owned(),
            ),
            (Stage::Responded(_, pendings), StepAnd::End { lower }) => {
                Self::end(&pendings, lower.to_owned())?
            }
            _ => {
                return Err(Error::Pair(
                    format!("{:?}", stage),
                    format!("{:?}", step_and),
                ));
            }
        };
        if stepping.variables().is_some_and(|v| v == variables) {
            return Err(Error::NoStep);
        }
        Ok(stepping)
    }

    fn cont(step: Cont, variables: Variables, bounds: Bounds) -> Self {
        let step_to = StepTo::Cont(step, Box::new(variables));
        Self { bounds, step_to }
    }

    fn eol(step: Eol, bounds: Bounds) -> Self {
        let step_to = StepTo::Eol(step);
        Self { bounds, step_to }
    }

    fn add(amount: u64, subbed: u64, useds: Vec<Used>, add: u64) -> Self {
        Self::cont(
            Cont::Add,
            Variables::new(amount + add, Stage::Opened(subbed, useds)),
            Bounds::default(),
        )
    }

    fn sub(amount: u64, subbed: u64, useds: Vec<Used>, receipt: Receipt, upper: Duration) -> Self {
        let (unlockeds, useds) = receipt.next_unlockeds_useds(&useds, &upper);
        let squash = receipt.squash.clone();
        let absolute_owed = squash.amount() + useds.iter().map(|u| u.amount).sum::<u64>();
        let relative_owed = absolute_owed.saturating_sub(subbed);
        let gain = cmp::min(relative_owed, amount);
        Self::cont(
            Cont::Sub(squash, unlockeds),
            Variables::new(amount - gain, Stage::Opened(subbed + gain, useds)),
            Bounds::upper(upper),
        )
    }

    fn close(
        amount: u64,
        subbed: u64,
        useds: Vec<Used>,
        upper: Duration,
        close_period: Duration,
    ) -> Self {
        let elapse_at = Duration(upper.saturating_add(*close_period));
        Self::cont(
            Cont::Close,
            Variables::new(amount, Stage::Closed(subbed, useds, elapse_at)),
            Bounds::upper(upper),
        )
    }

    fn respond(
        amount: u64,
        subbed: u64,
        useds: Vec<Used>,
        receipt: Receipt,
        upper: Duration,
    ) -> Self {
        let (cheques, pendings, useds_amount) =
            receipt.next_cheques_pendings_useds_amount(&useds, &upper);
        let pendings_amount = pendings.iter().map(|p| p.amount).sum::<u64>();
        let squash = receipt.squash.clone();
        let absolute_owed = squash.amount() + useds_amount;
        let relative_owed = absolute_owed.saturating_sub(subbed);
        let gain = cmp::min(relative_owed, amount);
        Self::cont(
            Cont::Respond(squash, cheques),
            Variables::new(amount - gain, Stage::Responded(pendings_amount, pendings)),
            Bounds::upper(upper),
        )
    }

    fn elapse(elapse_at: Duration, lower: Duration) -> Result<Self, Error> {
        if *elapse_at >= *lower {
            return Err(Error::Early(lower.clone(), elapse_at.clone()));
        };
        Ok(Self::eol(Eol::Elapse, Bounds::lower(lower)))
    }

    fn expire(pendings: Vec<Pending>, lower: Duration) -> Self {
        let filter = |p: &Pending| *p.timeout < *lower;
        let unpends = pendings
            .iter()
            .map(|p| {
                if filter(p) {
                    Unpend::Expire
                } else {
                    Unpend::Continue
                }
            })
            .collect::<Vec<_>>();
        let pendings = pendings.into_iter().filter(filter).collect::<Vec<_>>();
        let pendings_amount = pendings.iter().map(|p| p.amount).sum::<u64>();
        Self::cont(
            Cont::Expire(unpends),
            Variables::new(pendings_amount, Stage::Responded(pendings_amount, pendings)),
            Bounds::lower(lower),
        )
    }

    fn unlock(amount: u64, pendings: Vec<Pending>, secrets: Vec<Secret>, upper: Duration) -> Self {
        let lookup = secrets
            .into_iter()
            .map(|s| (Lock::from(&s), s))
            .collect::<BTreeMap<Lock, Secret>>();
        let unpends: Vec<Unpend> = pendings
            .iter()
            .map(|p| {
                if *p.timeout <= *upper {
                    // Timed out!
                    Unpend::Continue
                } else {
                    lookup.get(&p.lock).map_or(Unpend::Continue, Unpend::from)
                }
            })
            .collect::<Vec<_>>();
        let gain = pendings
            .iter()
            .zip(&unpends)
            .filter(|(_a, b)| !b.is_continue())
            .map(|(a, _b)| a.amount)
            .sum::<u64>();
        let pendings = pendings
            .iter()
            .zip(&unpends)
            .filter(|(_a, b)| !b.is_continue())
            .map(|(a, _b)| a.clone())
            .collect::<Vec<_>>();
        let pendings_amount = pendings.iter().map(|p| p.amount).sum::<u64>();
        Self::cont(
            Cont::Unlock(unpends),
            Variables::new(amount - gain, Stage::Responded(pendings_amount, pendings)),
            Bounds::upper(upper),
        )
    }

    fn end(pendings: &[Pending], lower: Duration) -> Result<Self, Error> {
        for pending in pendings {
            if *pending.timeout >= *lower {
                return Err(Error::Early(lower.clone(), pending.timeout));
            };
        }
        Ok(Self::eol(Eol::Elapse, Bounds::lower(lower)))
    }

    pub fn step(&self) -> Step {
        self.step_to.step()
    }

    pub fn bounds(&self) -> Bounds {
        self.bounds.clone()
    }

    pub fn variables(&self) -> Option<Variables> {
        self.step_to.variables()
    }
}
