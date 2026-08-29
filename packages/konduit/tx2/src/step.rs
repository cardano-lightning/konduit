use konduit_data::{
    Cheque, Constants, Cont, Duration, Eol, Secret, Squash, Stage, Step, Unlocked, Unpend,
    VerifyingKey,
};
use minicbor::{Decode, Encode};

use crate::channel::Channel;

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Can {
    #[n(0)]
    Add,
    #[n(1)]
    Sub {
        #[n(0)]
        available: u64,
    },
    #[n(2)]
    Close,
    #[n(3)]
    Respond {
        #[n(0)]
        before: Duration,
        #[n(1)]
        available: u64,
    },
    #[n(4)]
    End,
    #[n(5)]
    Elapse {
        #[n(0)]
        after: Duration,
    },
    #[n(6)]
    Unlock,
    #[n(7)]
    Expire,
}

// Want: flat and unconstrained — validation happens in `Channel::resolve`,
// not here.

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Want {
    #[n(0)]
    Add {
        #[n(0)]
        amount: u64,
    },
    #[n(1)]
    Sub {
        #[n(0)]
        squash: Squash,
        #[n(1)]
        cheques: Vec<Unlocked>,
    },
    #[n(2)]
    Close,
    #[n(3)]
    Respond {
        #[n(0)]
        squash: Squash,
        #[n(1)]
        cheques: Vec<Cheque>,
    },
    #[n(4)]
    End,
    #[n(5)]
    Elapse,
    #[n(6)]
    Unlock {
        #[n(0)]
        secrets: Vec<Secret>,
    },
    #[n(7)]
    Expire,
}

impl Want {
    pub fn label(&self) -> &'static str {
        match self {
            Want::Add { .. } => "Add",
            Want::Sub { .. } => "Sub",
            Want::Close => "Close",
            Want::Respond { .. } => "Respond",
            Want::End => "End",
            Want::Elapse => "Elapse",
            Want::Unlock { .. } => "Unlock",
            Want::Expire => "Expire",
        }
    }

    /// Batch path: derive an intent from a squash + cheques against the
    /// channel's current stage — funnels through `Channel::resolve` same
    /// as the interactive path. `squash` is unused for `Responded`
    /// (nothing left to sub/respond against); only the `Unlocked`
    /// cheques are relevant there.
    pub fn from_claim(stage: &Stage, squash: Squash, cheques: Vec<Cheque>) -> Want {
        match stage {
            Stage::Opened(..) => Want::Sub {
                squash,
                cheques: cheques
                    .into_iter()
                    .filter_map(|u| u.as_unlocked())
                    .collect(),
            },
            Stage::Closed(..) => Want::Respond { squash, cheques },
            Stage::Responded(..) => Want::Unlock {
                secrets: cheques
                    .into_iter()
                    .filter_map(|u| u.as_unlocked().map(|x| x.secret().to_owned()))
                    .collect(),
            },
        }
    }
}

// Will: Cont carries the continuing Channel; Eol carries none.

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Will {
    #[n(0)]
    Cont {
        #[n(0)]
        #[cbor(with = "crate::cbor_box")]
        output: Box<Channel>,
        #[n(1)]
        step: WillCont,
    },
    #[n(1)]
    Eol {
        #[n(0)]
        step: WillEol,
    },
}

impl Will {
    pub fn cont(output: Channel, step: WillCont) -> Self {
        Self::Cont {
            output: Box::new(output),
            step,
        }
    }

    pub fn eol(step: WillEol) -> Self {
        Self::Eol { step }
    }

    // `bounds()` is deliberately not here — `Channel::resolve` enforces
    // time against an `Interval` fixed upstream instead.

    pub fn to_step(&self) -> Step {
        match self {
            Will::Cont { step, .. } => Step::Cont(step.to_step()),
            Will::Eol { step } => Step::Eol(step.to_step()),
        }
    }

    pub fn is_adaptor(&self) -> bool {
        match self {
            Will::Cont { step, .. } => step.is_adaptor(),
            _ => false,
        }
    }

    pub fn signer(&self, constants: &Constants) -> VerifyingKey {
        if self.is_adaptor() {
            constants.sub_vkey.clone()
        } else {
            constants.add_vkey.clone()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum WillCont {
    #[n(0)]
    Add {
        #[n(0)]
        amount: u64,
    },
    #[n(1)]
    Sub {
        #[n(0)]
        squash: Squash,
        #[n(1)]
        unlockeds: Vec<Unlocked>,
        #[n(2)]
        gain: u64,
    },
    #[n(2)]
    Close,
    #[n(3)]
    Respond {
        #[n(0)]
        squash: Squash,
        #[n(1)]
        cheques: Vec<Cheque>,
        #[n(2)]
        gain: u64,
    },
    #[n(4)]
    Unlock {
        #[n(0)]
        unpends: Vec<Unpend>,
        #[n(1)]
        gain: u64,
    },
    #[n(5)]
    Expire {
        #[n(0)]
        unpends: Vec<Unpend>,
    },
}

impl WillCont {
    pub fn add(amount: u64) -> Self {
        Self::Add { amount }
    }
    pub fn sub(squash: Squash, unlockeds: Vec<Unlocked>, gain: u64) -> Self {
        Self::Sub {
            squash,
            unlockeds,
            gain,
        }
    }
    pub fn close() -> Self {
        Self::Close
    }
    pub fn respond(squash: Squash, cheques: Vec<Cheque>, gain: u64) -> Self {
        Self::Respond {
            squash,
            cheques,
            gain,
        }
    }
    pub fn unlock(unpends: Vec<Unpend>, gain: u64) -> Self {
        Self::Unlock { unpends, gain }
    }
    pub fn expire(unpends: Vec<Unpend>) -> Self {
        Self::Expire { unpends }
    }

    pub fn to_step(&self) -> Cont {
        match self {
            WillCont::Add { .. } => Cont::Add,
            WillCont::Sub {
                squash, unlockeds, ..
            } => Cont::Sub(squash.clone(), unlockeds.clone()),
            WillCont::Close => Cont::Close,
            WillCont::Respond {
                squash, cheques, ..
            } => Cont::Respond(squash.clone(), cheques.clone()),
            WillCont::Unlock { unpends, .. } => Cont::Unlock(unpends.clone()),
            WillCont::Expire { unpends } => Cont::Expire(unpends.clone()),
        }
    }

    fn is_adaptor(&self) -> bool {
        matches!(
            self,
            WillCont::Sub { .. } | WillCont::Respond { .. } | WillCont::Unlock { .. }
        )
    }
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum WillEol {
    #[n(0)]
    End,
    #[n(1)]
    Elapse {
        #[n(0)]
        lower: Duration,
    },
}

impl WillEol {
    pub fn end() -> Self {
        Self::End
    }
    pub fn elapse(lower: Duration) -> Self {
        Self::Elapse { lower }
    }

    pub fn to_step(&self) -> Eol {
        match self {
            WillEol::End => Eol::End,
            WillEol::Elapse { .. } => Eol::Elapse,
        }
    }
}

/// Per-channel: `Want` is invalid for this `Channel` (wrong stage, or
/// incompatible with the fixed interval). Squash/Cheque verification is
/// assumed to have already happened before a `Want` is constructed.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("no legal step: {from} cannot {to}")]
    Pair { from: String, to: &'static str },
    #[error("nothing to do")]
    NoStep,
    #[error("time bound infeasible: {reason}")]
    Bound { reason: &'static str },
}

impl Error {
    pub fn pair(from: impl Into<String>, to: &'static str) -> Self {
        Self::Pair {
            from: from.into(),
            to,
        }
    }
}
