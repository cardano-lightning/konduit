use konduit_data::{Duration, Pending, Stage, Unpend, Used};

use crate::StepError;

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

    pub fn close(&self, upper: &Duration, close_period: &Duration) -> Result<Self, StepError> {
        let Stage::Opened(subbed, used) = &self.stage else {
            return Err(StepError::pair(self.stage.label(), "Close"));
        };
        Ok(Self::new(
            self.amount,
            Stage::Closed(*subbed, used.clone(), *upper + *close_period),
        ))
    }

    pub fn elapse(&self, lower: &Duration) -> Result<(), StepError> {
        let Stage::Closed(_, _, elapse_at) = &self.stage else {
            return Err(StepError::pair(self.stage.label(), "Elapse"));
        };
        if lower >= elapse_at {
            Err(StepError::Early(*lower, *elapse_at))
        } else {
            Ok(())
        }
    }

    pub fn respond(&self, gain: u64, pendings: Vec<Pending>) -> Result<Self, StepError> {
        let Stage::Closed(_, _, _) = &self.stage else {
            return Err(StepError::pair(self.stage.label(), "Respond"));
        };
        let pendings_amount = pendings.iter().map(|x| x.amount).sum::<u64>();
        Ok(Self::new(
            self.amount - gain,
            Stage::Responded(pendings_amount, pendings),
        ))
    }

    pub fn unlock(&self, gain: u64, pendings: Vec<Pending>) -> Result<Self, StepError> {
        let Stage::Responded(_, _) = &self.stage else {
            return Err(StepError::pair(self.stage.label(), "Unlock"));
        };
        let pendings_amount = pendings.iter().map(|p| p.amount).sum::<u64>();
        Ok(Variables::new(
            self.amount.saturating_sub(gain),
            Stage::Responded(pendings_amount, pendings),
        ))
    }

    /// It is assumed that the pendings were derived sensibly by the caller.
    pub fn expire(&self, pendings: Vec<Pending>) -> Result<Self, StepError> {
        let Stage::Responded(_pendings_amount, _pendings) = &self.stage else {
            return Err(StepError::pair(self.stage.label(), "Expire"));
        };
        let pendings_amount = pendings.iter().map(|p| p.amount).sum::<u64>();
        Ok(Variables::new(
            pendings_amount,
            Stage::Responded(pendings_amount, pendings),
        ))
    }

    pub fn end(&self, lower: &Duration) -> Result<(), StepError> {
        todo!("Not yet implemented")
    }
}
