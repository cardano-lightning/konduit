use std::cmp::{self, min};

use konduit_data::{
    base::{Amount, Timestamp},
    stage::Stage,
    step::{Cont, Eol, Step},
};

use crate::{
    channel::Channel,
    constraints::Constraints,
    intent::{AdaptorIntent, ConsumerIntent, Intent},
};

#[derive(Debug, Clone)]
pub enum Stepped {
    Cont(Cont, Channel),
    Eol(Eol),
}

#[derive(Debug, Clone)]
pub enum CanStep {
    Yes(Stepped, Constraints),
    No,
    Bad(String),
}

impl CanStep {
    pub fn as_step(&self) -> Option<Step> {
        match self {
            CanStep::Yes(Stepped::Cont(cont, _), _) => Some(Step::Cont(cont.clone())),
            CanStep::Yes(Stepped::Eol(eol), _) => Some(Step::Eol(eol.clone())),
            _ => None,
        }
    }

    pub fn as_channel(&self) -> Option<Channel> {
        match self {
            CanStep::Yes(Stepped::Cont(_, channel), _) => Some(channel.clone()),
            _ => None,
        }
    }

    pub fn as_constraints(&self) -> Option<Constraints> {
        match self {
            CanStep::Yes(_, constriants) => Some(constriants.clone()),
            _ => None,
        }
    }

    pub fn from_channel_intent(mut channel: Channel, intent: Intent) -> CanStep {
        match intent {
            Intent::Consumer(consumer_intent) => {
                let constraints =
                    Constraints::default().with_required_signer(&channel.constants.add_vkey);
                match consumer_intent {
                    ConsumerIntent::Add(amount) => match channel.stage {
                        Stage::Opened(_) => {
                            channel.amount = Amount(channel.amount.0 + amount.0);
                            let stepped = Stepped::Cont(Cont::Add, channel);
                            CanStep::Yes(stepped, constraints)
                        }
                        _ => CanStep::Bad("Expect stage Opened".to_string()),
                    },
                    ConsumerIntent::Close(upper_bound) => match channel.stage {
                        Stage::Opened(subbed) => {
                            let elapse_at =
                                Timestamp(channel.constants.close_period.0 + upper_bound.0);
                            channel.stage = Stage::Closed(subbed, elapse_at);
                            let stepped = Stepped::Cont(Cont::Close, channel);
                            CanStep::Yes(stepped, constraints.with_upper_bound(upper_bound))
                        }
                        _ => CanStep::Bad("Expect stage Opened".to_string()),
                    },
                    ConsumerIntent::Timeout(lower_bound) => match channel.stage {
                        Stage::Closed(_, elapse_at) => {
                            if elapse_at.0 < lower_bound.0 {
                                let stepped = Stepped::Eol(Eol::Elapse);
                                CanStep::Yes(stepped, constraints.with_lower_bound(lower_bound))
                            } else {
                                CanStep::No
                            }
                        }
                        Stage::Responded(pending_amount_in, pendings) => {
                            let (unpends, release, remain) = pendings.expire(lower_bound);
                            if remain.0.len() == 0 {
                                let stepped = Stepped::Eol(Eol::End);
                                CanStep::Yes(stepped, constraints.with_lower_bound(lower_bound))
                            } else if remain.0.len() != pendings.0.len() {
                                let pending_amount_out = Amount(pending_amount_in.0 - release.0);
                                channel.amount = pending_amount_out;
                                channel.stage = Stage::Responded(pending_amount_out, remain);
                                let stepped = Stepped::Cont(Cont::Expire(unpends), channel);
                                CanStep::Yes(stepped, constraints.with_lower_bound(lower_bound))
                            } else {
                                CanStep::No
                            }
                        }
                        _ => CanStep::Bad("Expect stage not Opened".to_string()),
                    },
                }
            }
            Intent::Adaptor(adaptor_intent) => {
                let constraints =
                    Constraints::default().with_required_signer(&channel.constants.sub_vkey);
                match adaptor_intent {
                    AdaptorIntent::Sub(squash, unlockeds) => {
                        match channel.stage {
                            Stage::Opened(subbed_in) => {
                                let total_owed = squash.amount().0 + unlockeds.amount().0;
                                match total_owed.cmp(&subbed_in.0) {
                                    cmp::Ordering::Less => {
                                        CanStep::Bad("Insufficient evidence".to_string())
                                    }
                                    cmp::Ordering::Equal => CanStep::No,
                                    cmp::Ordering::Greater => {
                                        // FIXME :: No checking is done.
                                        let relative_owed = total_owed - subbed_in.0;
                                        let sub = min(relative_owed, channel.amount.0);
                                        let subbed_out = Amount(subbed_in.0 + sub);
                                        channel.stage = Stage::Opened(subbed_out);
                                        channel.amount = Amount(channel.amount.0 - sub);
                                        if let Some(upper_bound) = unlockeds.max_timeout() {
                                            let stepped = Stepped::Cont(
                                                Cont::Sub(squash, unlockeds),
                                                channel,
                                            );
                                            CanStep::Yes(
                                                stepped,
                                                constraints.with_upper_bound(upper_bound),
                                            )
                                        } else {
                                            let stepped = Stepped::Cont(
                                                Cont::Sub(squash, unlockeds),
                                                channel,
                                            );
                                            CanStep::Yes(stepped, constraints)
                                        }
                                    }
                                }
                            }
                            _ => CanStep::Bad("Expect stage Opened".to_string()),
                        }
                    }
                    AdaptorIntent::Respond(squash, mixed_cheques) => {
                        match channel.stage {
                            Stage::Closed(subbed_in, _) => {
                                let total_owed = squash.amount().0 + mixed_cheques.amount().0;
                                let (pending_amount, pendings) = mixed_cheques.pendings();
                                match total_owed.cmp(&subbed_in.0) {
                                    cmp::Ordering::Less => {
                                        CanStep::Bad("Insufficient evidence".to_string())
                                    }
                                    _ => {
                                        // This could be a no, but we're nice!
                                        // FIXME :: No checking is done.
                                        let relative_owed = total_owed - subbed_in.0;
                                        let sub = min(relative_owed, channel.amount.0);
                                        channel.amount = Amount(channel.amount.0 - sub);
                                        channel.stage = Stage::Responded(pending_amount, pendings);
                                        if let Some(upper_bound) = mixed_cheques.max_timeout() {
                                            let stepped = Stepped::Cont(
                                                Cont::Respond(squash, mixed_cheques),
                                                channel,
                                            );
                                            CanStep::Yes(
                                                stepped,
                                                constraints.with_upper_bound(upper_bound),
                                            )
                                        } else {
                                            let stepped = Stepped::Cont(
                                                Cont::Respond(squash, mixed_cheques),
                                                channel,
                                            );
                                            CanStep::Yes(stepped, constraints)
                                        }
                                    }
                                }
                            }
                            _ => CanStep::Bad("Expect stage Closed".to_string()),
                        }
                    }
                    AdaptorIntent::Unlock(secrets, upper_bound) => match channel.stage {
                        Stage::Responded(pending_amount_in, pendings) => {
                            let (unpends, release, remain) = pendings.unlock(secrets, upper_bound);
                            if remain.0.len() != pendings.0.len() {
                                let pending_amount_out = Amount(pending_amount_in.0 - release.0);
                                channel.amount = Amount(channel.amount.0 - release.0);
                                channel.stage = Stage::Responded(pending_amount_out, remain);
                                let stepped = Stepped::Cont(Cont::Unlock(unpends), channel);
                                CanStep::Yes(stepped, constraints.with_upper_bound(upper_bound))
                            } else {
                                CanStep::No
                            }
                        }
                        _ => CanStep::Bad("Expect stage Responded".to_string()),
                    },
                }
            }
        }
    }
}
