use crate::{ChequeBody, Duration, Lock, Tag, locked::Locked, unlocked::Unlocked};
use anyhow::anyhow;
use cardano_sdk::{PlutusData, VerificationKey};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cheque {
    Unlocked(Unlocked),
    Locked(Locked),
}

impl PartialOrd for Cheque {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Cheque {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.index().cmp(&other.index()) {
            ordering @ std::cmp::Ordering::Less | ordering @ std::cmp::Ordering::Greater => {
                ordering
            }
            std::cmp::Ordering::Equal => match (self, other) {
                (Self::Unlocked(a), Self::Unlocked(b)) => a.cmp(b),
                (Self::Locked(a), Self::Locked(b)) => a.cmp(b),
                (Self::Unlocked(_), _) => std::cmp::Ordering::Greater,
                (Self::Locked(_), _) => std::cmp::Ordering::Less,
            },
        }
    }
}

impl Cheque {
    pub fn is_cheque(&self) -> bool {
        !matches!(self, &Self::Unlocked(_))
    }

    pub fn body(&self) -> &ChequeBody {
        match self {
            Self::Unlocked(unlocked) => &unlocked.body,
            Self::Locked(locked) => &locked.body,
        }
    }

    pub fn index(&self) -> u64 {
        self.body().index
    }

    pub fn amount(&self) -> u64 {
        self.body().amount
    }

    pub fn lock(&self) -> &Lock {
        &self.body().lock
    }

    pub fn timeout(&self) -> &Duration {
        &self.body().timeout
    }

    pub fn as_unlocked(&self) -> Option<Unlocked> {
        match self {
            Cheque::Unlocked(unlocked) => Some(unlocked.clone()),
            Cheque::Locked(_) => None,
        }
    }

    pub fn as_locked(&self) -> Option<Locked> {
        match self {
            Cheque::Unlocked(_) => None,
            Cheque::Locked(locked) => Some(locked.clone()),
        }
    }

    pub fn verify(&self, key: &VerificationKey, tag: &Tag) -> bool {
        match self {
            Cheque::Unlocked(unlocked) => unlocked.verify_no_time(key, tag),
            Cheque::Locked(cheque) => cheque.verify(key, tag),
        }
    }
}

impl From<Unlocked> for Cheque {
    fn from(value: Unlocked) -> Self {
        Cheque::Unlocked(value)
    }
}

impl From<Locked> for Cheque {
    fn from(value: Locked) -> Self {
        Cheque::Locked(value)
    }
}

impl<'a> TryFrom<PlutusData<'a>> for Cheque {
    type Error = anyhow::Error;

    fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
        let (variant, fields): (u64, Vec<PlutusData<'_>>) = (&data).try_into()?;

        return match variant {
            _ if variant == 0 => {
                try_unlocked(fields).map_err(|e| e.context("invalid 'Unlocked' variant"))
            }
            _ if variant == 1 => {
                try_cheque(fields).map_err(|e| e.context("invalid 'Cheque' variant"))
            }
            _ => Err(anyhow!("unknown variant: {variant}")),
        };

        fn try_unlocked(fields: Vec<PlutusData<'_>>) -> anyhow::Result<Cheque> {
            Ok(Cheque::from(Unlocked::try_from(fields)?))
        }

        fn try_cheque(fields: Vec<PlutusData<'_>>) -> anyhow::Result<Cheque> {
            Ok(Cheque::from(Locked::try_from(fields)?))
        }
    }
}

impl<'a> From<Cheque> for PlutusData<'a> {
    fn from(value: Cheque) -> Self {
        match value {
            Cheque::Unlocked(unlocked) => PlutusData::constr(0, Vec::from(unlocked)),
            Cheque::Locked(locked) => PlutusData::constr(1, Vec::from(locked)),
        }
    }
}
