use cardano_sdk::PosixMilliseconds;
use konduit_data::{Duration, Pending, Stage, Used};

use crate::StepError;

/// A duration in milliseconds, saturated to what a u64 can hold.
fn millis(duration: &Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

/// Channel Variables aka Channel Data but without the constants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variables {
    amount: u64,
    stage: Stage,
}

impl Variables {
    pub fn new(amount: u64, stage: Stage) -> Self {
        Self { amount, stage }
    }

    pub fn amount(&self) -> u64 {
        self.amount
    }

    pub fn stage(&self) -> &Stage {
        &self.stage
    }

    pub fn add(&self, amount: u64) -> Result<Self, StepError> {
        let Stage::Opened(_, _) = &self.stage else {
            return Err(StepError::pair(self.stage.label(), "Add"));
        };
        if amount == 0 {
            return Err(StepError::NoStep);
        }
        Ok(Self::new(self.amount + amount, self.stage.clone()))
    }

    pub fn sub(&self, gain: u64, useds: Vec<Used>) -> Result<Self, StepError> {
        let Stage::Opened(subbed, _) = &self.stage else {
            return Err(StepError::pair(self.stage.label(), "Sub"));
        };
        Ok(Self::new(
            self.amount - gain,
            Stage::Opened(subbed + gain, useds),
        ))
    }

    pub fn close(
        &self,
        upper: &PosixMilliseconds,
        close_period: &Duration,
    ) -> Result<Self, StepError> {
        let Stage::Opened(subbed, used) = &self.stage else {
            return Err(StepError::pair(self.stage.label(), "Close"));
        };
        Ok(Self::new(
            self.amount,
            Stage::Closed(
                *subbed,
                used.clone(),
                // The datum carries the elapse time as a duration in milliseconds.
                Duration::from_millis(upper.as_millis().saturating_add(millis(close_period))),
            ),
        ))
    }

    pub fn elapse(&self, lower: &PosixMilliseconds) -> Result<(), StepError> {
        let Stage::Closed(_, _, elapse_at) = &self.stage else {
            return Err(StepError::pair(self.stage.label(), "Elapse"));
        };
        if u128::from(lower.as_millis()) <= elapse_at.as_millis() {
            Err(StepError::Early(
                Duration::from_millis(lower.as_millis()),
                *elapse_at,
            ))
        } else {
            Ok(())
        }
    }

    pub fn respond(&self, gain: u64, pendings: Vec<Pending>) -> Result<Self, StepError> {
        let Stage::Closed(_, _, _) = &self.stage else {
            return Err(StepError::pair(self.stage.label(), "Respond"));
        };
        let pends_amount = pendings.iter().map(|x| x.amount).sum::<u64>();
        Ok(Self::new(
            self.amount - gain,
            Stage::Responded(pends_amount, pendings),
        ))
    }

    pub fn unlock(&self, gain: u64, pendings: Vec<Pending>) -> Result<Self, StepError> {
        let Stage::Responded(_, _) = &self.stage else {
            return Err(StepError::pair(self.stage.label(), "Unlock"));
        };
        let pends_amount = pendings.iter().map(|p| p.amount).sum::<u64>();
        Ok(Variables::new(
            self.amount.saturating_sub(gain),
            Stage::Responded(pends_amount, pendings),
        ))
    }

    /// It is assumed that the pendings were derived sensibly by the caller.
    pub fn expire(&self, pendings: Vec<Pending>) -> Result<Self, StepError> {
        let Stage::Responded(_pends_amount, _pendings) = &self.stage else {
            return Err(StepError::pair(self.stage.label(), "Expire"));
        };
        let pends_amount = pendings.iter().map(|p| p.amount).sum::<u64>();
        Ok(Variables::new(
            pends_amount,
            Stage::Responded(pends_amount, pendings),
        ))
    }

    pub fn end(&self, _lower: Option<&Duration>) -> Result<(), StepError> {
        panic!("Use `Channel::end` instead.")
    }
}
