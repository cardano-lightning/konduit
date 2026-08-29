use std::{cmp, collections::BTreeMap};

use minicbor::{Decode, Encode};

use cardano_sdk::{Credential, Output, Value};
use konduit_data::{
    Cheque, Constants, Datum, Duration, Lock, Pending, Secret, Squash, Stage, Unlocked, Unpend,
    Used,
};

use crate::{
    Interval, KONDUIT_VALIDATOR, MIN_ADA_BUFFER,
    currency::Currency,
    step::{Can, Error as StepError, Want, Will, WillCont, WillEol},
};

#[derive(Debug, Clone, thiserror::Error)]
pub enum FromOutputError {
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Channel {
    #[n(0)]
    delegation: Option<Credential>,
    #[n(1)]
    constants: Constants,
    #[n(2)]
    amount: u64,
    #[n(3)]
    stage: Stage,
    /// Always `Ada` for now
    #[n(4)]
    currency: Currency,
}

impl TryFrom<&Output> for Channel {
    type Error = FromOutputError;

    fn try_from(output: &Output) -> Result<Self, Self::Error> {
        let Some(address) = output.address().as_shelley() else {
            return Err(FromOutputError::ShelleyAddress);
        };
        let Some(hash) = address.payment().as_script() else {
            return Err(FromOutputError::ScriptCredential);
        };
        if hash != KONDUIT_VALIDATOR.hash {
            return Err(FromOutputError::KonduitCredential);
        }
        let delegation: Option<Credential> = address.delegation().clone();

        let Some(datum) = output.datum() else {
            return Err(FromOutputError::Datum);
        };
        let cardano_sdk::Datum::Inline(data) = datum else {
            return Err(FromOutputError::Inline);
        };
        let Datum {
            own_hash,
            constants,
            stage,
        } = Datum::try_from(data).map_err(|_| FromOutputError::ParseDatum)?;
        if own_hash != <[u8; 28]>::from(KONDUIT_VALIDATOR.hash) {
            return Err(FromOutputError::OwnHash);
        }

        let amount = debuffer_amount(output.value());
        Ok(Self {
            delegation,
            constants,
            amount,
            stage,
            currency: Currency::Ada,
        })
    }
}

pub fn debuffer_amount(value: &cardano_sdk::Value<u64>) -> u64 {
    value.lovelace().saturating_sub(MIN_ADA_BUFFER)
}

// ---------------------------------------------------------------------------
// Prep helpers: sub/respond both filter cheques against a `Squash` +
// `useds` history; shared here to avoid duplicating that filter.
// ---------------------------------------------------------------------------

/// Cheques still active as of `upper`: not expired, not already in `useds`.
fn active_unlockeds<'a>(
    cheques: &'a [Unlocked],
    useds: &'a [Used],
    upper: &'a Duration,
) -> impl Iterator<Item = &'a Unlocked> {
    cheques
        .iter()
        .filter(move |c| c.timeout() > *upper && !useds.iter().any(|x| x.index == c.index()))
}

fn active_cheques<'a>(
    cheques: &'a [Cheque],
    useds: &'a [Used],
    upper: &'a Duration,
) -> impl Iterator<Item = &'a Cheque> {
    cheques
        .iter()
        .filter(move |c| c.timeout() > *upper && !useds.iter().any(|x| x.index == c.index()))
}

/// Useds not yet folded into `squash` — still owed.
fn owed_useds<'a>(useds: &'a [Used], squash: &'a Squash) -> impl Iterator<Item = &'a Used> {
    useds.iter().filter(|u| !squash.is_index_squashed(u.index))
}

/// Spendable (unlocked) items for the next tx, plus the running used-history.
fn prep_sub(
    squash: &Squash,
    cheques: &[Unlocked],
    useds: &[Used],
    upper: &Duration,
) -> (Vec<Unlocked>, Vec<Used>) {
    let unlockeds: Vec<Unlocked> = active_unlockeds(cheques, useds, upper).cloned().collect();
    let mut next_useds: Vec<Used> = owed_useds(useds, squash)
        .cloned()
        .chain(unlockeds.iter().map(Used::from))
        .collect();
    next_useds.sort_by_key(|u| u.index);
    (unlockeds, next_useds)
}

/// Active cheques, split into pendings (locked) and amount owed (unlocked).
fn prep_respond(
    squash: &Squash,
    cheques: &[Cheque],
    useds: &[Used],
    upper: &Duration,
) -> (Vec<Cheque>, Vec<Pending>, u64) {
    let out_cheques: Vec<Cheque> = active_cheques(cheques, useds, upper).cloned().collect();

    let mut pendings = Vec::new();
    let mut unlocked_amount = 0u64;
    for c in &out_cheques {
        match c {
            Cheque::Locked(locked) => pendings.push(Pending::from(locked.clone())),
            Cheque::Unlocked(unlocked) => unlocked_amount += unlocked.amount(),
        }
    }

    let total_amount = owed_useds(useds, squash).map(|u| u.amount).sum::<u64>() + unlocked_amount;

    (out_cheques, pendings, total_amount)
}

impl Channel {
    /// The only constructor. `TryFrom<&Output>` is parsing, not construction.
    pub fn new_ada(
        delegation: Option<Credential>,
        constants: Constants,
        amount: u64,
        stage: Stage,
    ) -> Self {
        Self {
            delegation,
            constants,
            amount,
            stage,
            currency: Currency::Ada,
        }
    }

    /// Sugar over `new_ada` for a fresh channel. `new_ada` still takes an
    /// arbitrary `Stage` directly for starting mid-lifecycle (useful in tests).
    pub fn new_open(delegation: Option<Credential>, constants: Constants, amount: u64) -> Self {
        Self::new_ada(delegation, constants, amount, Stage::Opened(0, vec![]))
    }

    /// Continuing `Channel` for `resolve`: delegation/constants/currency
    /// carried forward, only amount/stage transition.
    fn stepped(&self, amount: u64, stage: Stage) -> Self {
        Self {
            delegation: self.delegation.clone(),
            constants: self.constants.clone(),
            amount,
            stage,
            currency: self.currency.clone(),
        }
    }

    pub fn delegation(&self) -> &Option<Credential> {
        &self.delegation
    }

    pub fn constants(&self) -> &Constants {
        &self.constants
    }

    pub fn currency(&self) -> &Currency {
        &self.currency
    }

    pub fn stage(&self) -> &Stage {
        &self.stage
    }

    pub fn amount(&self) -> u64 {
        self.amount
    }

    /// Ada channels require min ada buffer
    pub fn buffered_amount(&self) -> u64 {
        self.amount() + MIN_ADA_BUFFER
    }

    pub fn buffered_value(&self) -> Value<u64> {
        Value::new(self.buffered_amount())
    }

    pub fn datum(&self) -> Datum {
        Datum {
            own_hash: <[u8; 28]>::from(KONDUIT_VALIDATOR.hash),
            constants: self.constants.clone(),
            stage: self.stage().clone(),
        }
    }

    fn elapse_at(&self) -> Option<Duration> {
        match self.stage {
            Stage::Closed(_, _, elapse_at) => Some(elapse_at),
            _ => None,
        }
    }

    // Advisory: reads the same stored deadlines `resolve` checks, so
    // there's nothing for `can` and `resolve` to disagree on.

    pub fn can(&self) -> Vec<Can> {
        match &self.stage {
            Stage::Opened(_subbed, _useds) => {
                vec![
                    Can::Add,
                    Can::Sub {
                        available: self.amount(),
                    },
                    Can::Close,
                ]
            }
            Stage::Closed(_, _, elapse_at) => vec![
                Can::Respond {
                    before: *elapse_at,
                    available: self.amount(),
                },
                Can::Elapse { after: *elapse_at },
            ],
            Stage::Responded(_pendings_amount, _pendings) => {
                vec![Can::Unlock, Can::Expire, Can::End]
            }
        }
    }

    /// The single transition entry point, replacing a per-`Want` method.
    pub fn resolve(&self, want: Want, interval: &Interval) -> Result<Will, StepError> {
        match want {
            Want::Add { amount } => self.want_add(amount),
            Want::Sub { squash, cheques } => self.want_sub(&squash, &cheques, interval),
            Want::Close => self.want_close(interval),
            Want::Respond { squash, cheques } => self.want_respond(&squash, &cheques, interval),
            Want::End => self.want_end(interval),
            Want::Elapse => self.want_elapse(interval),
            Want::Unlock { secrets } => self.want_unlock(&secrets, interval),
            Want::Expire => self.want_expire(interval),
        }
    }

    fn want_add(&self, amount: u64) -> Result<Will, StepError> {
        let Stage::Opened(_, _) = &self.stage else {
            return Err(StepError::pair(self.stage.label(), "Add"));
        };
        if amount == 0 {
            return Err(StepError::NoStep);
        }
        let output = self.stepped(self.amount + amount, self.stage.clone());
        Ok(Will::cont(output, WillCont::add(amount)))
    }

    /// `upper` is the interval's own upper bound — `prep_sub` uses it to
    /// decide which cheques are still spendable when this tx executes.
    fn want_sub(
        &self,
        squash: &Squash,
        cheques: &[Unlocked],
        interval: &Interval,
    ) -> Result<Will, StepError> {
        let Stage::Opened(subbed, useds) = &self.stage else {
            return Err(StepError::pair(self.stage.label(), "Sub"));
        };
        let Some(upper) = interval.upper else {
            return Err(StepError::Bound {
                reason: "sub requires an interval upper bound",
            });
        };
        let (unlockeds, useds) = prep_sub(squash, cheques, useds, &upper);
        let absolute_owed = squash.amount() + useds.iter().map(|u| u.amount).sum::<u64>();
        let relative_owed = absolute_owed.saturating_sub(*subbed);
        let gain = cmp::min(relative_owed, self.amount());
        if gain == 0 {
            return Err(StepError::NoStep);
        }
        let output = self.stepped(self.amount - gain, Stage::Opened(subbed + gain, useds));
        Ok(Will::cont(
            output,
            WillCont::sub(squash.clone(), unlockeds, gain),
        ))
    }

    fn want_close(&self, interval: &Interval) -> Result<Will, StepError> {
        let Stage::Opened(subbed, used) = &self.stage else {
            return Err(StepError::pair(self.stage.label(), "Close"));
        };
        let Some(upper) = interval.upper else {
            return Err(StepError::Bound {
                reason: "close requires an interval upper bound",
            });
        };
        let elapse_at = upper + self.constants().close_period;
        let output = self.stepped(self.amount, Stage::Closed(*subbed, used.clone(), elapse_at));
        Ok(Will::cont(output, WillCont::close()))
    }

    /// Mirrors `Can::Respond`'s `before` constraint: interval upper must sit
    /// strictly before `elapse_at`. Same value doubles as `prep_respond`'s
    /// `upper`.
    fn want_respond(
        &self,
        squash: &Squash,
        cheques: &[Cheque],
        interval: &Interval,
    ) -> Result<Will, StepError> {
        let Stage::Closed(subbed, useds, elapse_at) = &self.stage else {
            return Err(StepError::pair(self.stage.label(), "Respond"));
        };
        let Some(upper) = interval.upper else {
            return Err(StepError::Bound {
                reason: "respond requires an interval upper bound",
            });
        };
        if upper >= *elapse_at {
            return Err(StepError::Bound {
                reason: "respond interval not strictly before elapse_at",
            });
        }
        let (out_cheques, pendings, useds_amount) = prep_respond(squash, cheques, useds, &upper);
        let absolute_owed = squash.amount() + useds_amount;
        let relative_owed = absolute_owed.saturating_sub(*subbed);
        let gain = cmp::min(relative_owed, self.amount());
        let pendings_amount = pendings.iter().map(|p| p.amount).sum::<u64>();
        let output = self.stepped(
            self.amount - gain,
            Stage::Responded(pendings_amount, pendings),
        );
        Ok(Will::cont(
            output,
            WillCont::respond(squash.clone(), out_cheques, gain),
        ))
    }

    /// Mirrors `Can::Elapse`'s `after` constraint: interval lower must sit
    /// strictly after `elapse_at`.
    fn want_elapse(&self, interval: &Interval) -> Result<Will, StepError> {
        let Some(elapse_at) = self.elapse_at() else {
            return Err(StepError::pair(self.stage.label(), "Elapse"));
        };
        let Some(lower) = interval.lower else {
            return Err(StepError::Bound {
                reason: "elapse requires an interval lower bound",
            });
        };
        if lower <= elapse_at {
            return Err(StepError::Bound {
                reason: "elapse lower not strictly after elapse_at",
            });
        }
        Ok(Will::eol(WillEol::elapse(lower)))
    }

    fn want_end(&self, interval: &Interval) -> Result<Will, StepError> {
        let Stage::Responded(_pendings_amount, pendings) = self.stage() else {
            return Err(StepError::pair(self.stage.label(), "End"));
        };
        if !pendings.is_empty() {
            let Some(lower) = interval.lower else {
                return Err(StepError::Bound {
                    reason: "end with pending secrets requires an interval lower bound",
                });
            };
            for pending in pendings {
                if pending.timeout >= lower {
                    return Err(StepError::Bound {
                        reason: "pending not yet timed out for interval lower",
                    });
                }
            }
        }
        Ok(Will::eol(WillEol::end()))
    }

    /// A pending only resolves via secret once its own timeout has passed
    /// (`p.timeout < upper`) — before that it simply continues, even if a
    /// matching secret was supplied.
    fn want_unlock(&self, secrets: &[Secret], interval: &Interval) -> Result<Will, StepError> {
        let Stage::Responded(_pendings_amount, pendings) = self.stage() else {
            return Err(StepError::pair(self.stage.label(), "Unlock"));
        };
        let Some(upper) = interval.upper else {
            return Err(StepError::Bound {
                reason: "unlock requires an interval upper bound",
            });
        };
        let lookup: BTreeMap<Lock, Secret> =
            secrets.iter().map(|u| (Lock::from(u), u.clone())).collect();
        let unpend = |p: &Pending| {
            if p.timeout >= upper {
                Unpend::Continue
            } else {
                lookup.get(&p.lock).map_or(Unpend::Continue, Unpend::from)
            }
        };
        let unpends: Vec<Unpend> = pendings.iter().map(unpend).collect();
        let gain = pendings
            .iter()
            .zip(&unpends)
            .filter(|(_, u)| !u.is_continue())
            .map(|(p, _)| p.amount)
            .sum::<u64>();
        let remaining: Vec<Pending> = pendings
            .iter()
            .zip(&unpends)
            .filter(|(_, u)| u.is_continue())
            .map(|(p, _)| p.clone())
            .collect();
        let pendings_amount = remaining.iter().map(|p| p.amount).sum::<u64>();
        let output = self.stepped(
            self.amount.saturating_sub(gain),
            Stage::Responded(pendings_amount, remaining),
        );
        Ok(Will::cont(output, WillCont::unlock(unpends, gain)))
    }

    /// Pendings timed out (`timeout < lower`) are dropped; the channel's
    /// own amount resets to whatever's left, since that's now the true
    /// total the continuing output holds.
    fn want_expire(&self, interval: &Interval) -> Result<Will, StepError> {
        let Stage::Responded(_pendings_amount, pendings) = self.stage() else {
            return Err(StepError::pair(self.stage.label(), "Expire"));
        };
        let Some(lower) = interval.lower else {
            return Err(StepError::Bound {
                reason: "expire requires an interval lower bound",
            });
        };
        let unpend = |p: &Pending| {
            if p.timeout < lower {
                Unpend::Expire
            } else {
                Unpend::Continue
            }
        };
        let unpends: Vec<Unpend> = pendings.iter().map(unpend).collect();
        if unpends.iter().all(Unpend::is_continue) {
            return Err(StepError::NoStep);
        }
        let remaining: Vec<Pending> = pendings
            .iter()
            .zip(&unpends)
            .filter(|(_, u)| u.is_continue())
            .map(|(p, _)| p.clone())
            .collect();
        let pendings_amount = remaining.iter().map(|p| p.amount).sum::<u64>();
        let output = self.stepped(
            pendings_amount,
            Stage::Responded(pendings_amount, remaining),
        );
        Ok(Will::cont(output, WillCont::expire(unpends)))
    }
}
