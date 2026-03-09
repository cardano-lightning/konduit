use std::{cmp, collections::BTreeMap};

use cardano_sdk::{Output, Value};
use konduit_data::{
    Constants, Cont, Datum, Duration, Eol, Lock, Pending, Receipt, Secret, Stage, Unpend,
};

use crate::{
    Bounds, KONDUIT_VALIDATOR, MIN_ADA_BUFFER, StepError, StepTo, Stepped, variables::Variables,
};

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("Expect Shelley Address")]
    ShelleyAddress,
    #[error("Expect Script Payment Credential")]
    ScriptCredential,
    #[error("Expect Konduit Payment Credential")]
    KonduitCredential,
    #[error("Expect datum")]
    Datum,
    #[error("Expect Inline datum")]
    Inline,
    #[error("Failed to parse datum")]
    ParseDatum,
    #[error("Own hash is wrong")]
    OwnHash,
}

impl TryFrom<&Output> for Channel {
    type Error = Error;

    fn try_from(output: &Output) -> Result<Self, Self::Error> {
        let Some(address) = output.address().as_shelley() else {
            return Err(Error::ShelleyAddress);
        };
        let Some(hash) = address.payment().as_script() else {
            return Err(Error::ScriptCredential);
        };
        if hash != KONDUIT_VALIDATOR.hash {
            return Err(Error::KonduitCredential);
        }
        let Some(datum) = output.datum() else {
            return Err(Error::Datum);
        };
        let cardano_sdk::Datum::Inline(data) = datum else {
            return Err(Error::Inline);
        };
        let Datum {
            own_hash,
            constants,
            stage,
        } = Datum::try_from(data).map_err(|_| Error::ParseDatum)?;
        if own_hash != KONDUIT_VALIDATOR.hash {
            return Err(Error::OwnHash);
        }
        let amount = debuffer_amount(output.value());
        let variables = Variables::new(amount, stage);
        Ok(Self {
            constants,
            variables,
        })
    }
}

pub fn debuffer_amount(value: &cardano_sdk::Value<u64>) -> u64 {
    value.lovelace().saturating_sub(MIN_ADA_BUFFER)
}

/// Data obtained from parsing a channel
#[derive(Debug, Clone)]
pub struct Channel {
    constants: Constants,
    variables: Variables,
}

type SteppedElseChannel = Result<Stepped, (Channel, StepError)>;

impl Channel {
    pub fn new(constants: Constants, variables: Variables) -> Self {
        Self {
            constants,
            variables,
        }
    }

    pub fn constants(&self) -> &Constants {
        &self.constants
    }

    pub fn variables(&self) -> &Variables {
        &self.variables
    }

    pub fn stage(&self) -> &Stage {
        &self.variables.stage()
    }

    pub fn amount(&self) -> u64 {
        self.variables.amount()
    }

    /// Ada channels require min ada buffer
    pub fn buffered_amount(&self) -> u64 {
        self.amount() + MIN_ADA_BUFFER
    }

    /// Ada channels require min ada buffer
    pub fn buffered_value(&self) -> Value<u64> {
        Value::new(self.buffered_amount())
    }

    /// As datum
    pub fn datum(&self) -> Datum {
        Datum {
            own_hash: KONDUIT_VALIDATOR.hash,
            constants: self.constants.clone(),
            stage: self.stage().clone(),
        }
    }

    pub fn add(self, amount: u64) -> SteppedElseChannel {
        let variables = match self.variables.add(amount) {
            Ok(variables) => variables,
            Err(err) => return Err((self, err)),
        };
        let step_to = StepTo::cont(Cont::Add, variables);
        Ok(Stepped::new(self, step_to, Bounds::default()))
    }

    pub fn sub(self, receipt: &Receipt, upper: &Duration) -> SteppedElseChannel {
        let Stage::Opened(subbed, useds) = self.stage() else {
            let label = self.stage().label().to_string();
            return Err((self, StepError::pair(label, "Sub")));
        };
        let (unlockeds, useds) = receipt.next_unlockeds_useds(&useds, upper);
        let squash = receipt.squash.clone();
        let absolute_owed = squash.amount() + useds.iter().map(|u| u.amount).sum::<u64>();
        let relative_owed = absolute_owed.saturating_sub(*subbed);
        let gain = cmp::min(relative_owed, self.amount());
        if gain == 0 {
            return Err((self, StepError::NoStep));
        }
        // It ought to be impossible to fail
        let variables = match self.variables.sub(gain, useds) {
            Ok(variables) => variables,
            Err(err) => return Err((self, err)),
        };
        let step_to = StepTo::cont(Cont::Sub(squash, unlockeds), variables);
        Ok(Stepped::new(self, step_to, Bounds::upper(*upper)))
    }

    fn close(self, upper: &Duration) -> SteppedElseChannel {
        let variables = match self.variables.close(upper, &self.constants().close_period) {
            Ok(variables) => variables,
            Err(err) => return Err((self, err)),
        };
        let step_to = StepTo::cont(Cont::Close, variables);
        Ok(Stepped::new(self, step_to, Bounds::upper(*upper)))
    }

    fn elapse(self, lower: &Duration) -> SteppedElseChannel {
        if let Err(err) = self.variables.elapse(lower) {
            return Err((self, err));
        };
        Ok(Stepped::new(
            self,
            StepTo::eol(Eol::Elapse),
            Bounds::lower(*lower),
        ))
    }

    pub fn respond(self, receipt: &Receipt, upper: &Duration) -> SteppedElseChannel {
        let Stage::Closed(subbed, useds, _) = self.stage() else {
            let label = self.stage().label().to_string();
            return Err((self, StepError::pair(label, "Respond")));
        };
        let (cheques, pendings, useds_amount) =
            receipt.next_cheques_pendings_useds_amount(&useds, &upper);
        let squash = receipt.squash.clone();
        let absolute_owed = squash.amount() + useds_amount;
        let relative_owed = absolute_owed.saturating_sub(*subbed);
        let gain = cmp::min(relative_owed, self.amount());
        // It ought to be impossible to fail
        let variables = match self.variables.respond(gain, pendings) {
            Ok(variables) => variables,
            Err(err) => return Err((self, err)),
        };
        let step_to = StepTo::cont(Cont::Respond(squash, cheques), variables);
        Ok(Stepped::new(self, step_to, Bounds::upper(*upper)))
    }

    pub fn unlock(self, receipt: &Receipt, upper: &Duration) -> SteppedElseChannel {
        let secrets = receipt
            .unlockeds()
            .into_iter()
            .map(|u| u.secret)
            .collect::<Vec<_>>();
        self.unlock_with_secrets(secrets, upper)
    }

    pub fn unlock_with_secrets(self, secrets: Vec<Secret>, upper: &Duration) -> SteppedElseChannel {
        let Stage::Responded(_pendings_amount, pendings) = self.stage() else {
            let label = self.stage().label().to_string();
            return Err((self, StepError::pair(label, "Unlock")));
        };
        let lookup = secrets
            .into_iter()
            .map(|s| (Lock::from(&s), s))
            .collect::<BTreeMap<Lock, Secret>>();
        let map = |p: &Pending| {
            if p.timeout >= *upper {
                Unpend::Continue
            } else {
                lookup.get(&p.lock).map_or(Unpend::Continue, Unpend::from)
            }
        };
        let unpends: Vec<Unpend> = pendings.iter().map(map).collect::<Vec<_>>();
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
        // It ought to be impossible to fail
        let variables = match self.variables.unlock(gain, pendings) {
            Ok(variables) => variables,
            Err(err) => return Err((self, err)),
        };
        let step_to = StepTo::cont(Cont::Unlock(unpends), variables);
        Ok(Stepped::new(self, step_to, Bounds::upper(*upper)))
    }

    pub fn expire(self, lower: &Duration) -> SteppedElseChannel {
        let Stage::Responded(_pendings_amount, pendings) = &self.stage() else {
            let label = self.stage().label().to_string();
            return Err((self, StepError::pair(label, "Expire")));
        };
        let map = |p: &Pending| {
            if p.timeout < *lower {
                Unpend::Expire
            } else {
                Unpend::Continue
            }
        };
        let unpends = pendings.iter().map(map).collect::<Vec<_>>();
        if unpends.iter().all(|x| *x == Unpend::Continue) {
            return Err((self, StepError::NoStep));
        };
        let pendings = pendings
            .iter()
            .zip(&unpends)
            .filter(|(_, b)| b == &&Unpend::Continue)
            .map(|a| a.0)
            .cloned()
            .collect::<Vec<_>>();
        let variables = match self.variables.expire(pendings) {
            Ok(variables) => variables,
            Err(err) => return Err((self, err)),
        };
        let step_to = StepTo::cont(Cont::Expire(unpends), variables);
        Ok(Stepped::new(self, step_to, Bounds::lower(*lower)))
    }
}
