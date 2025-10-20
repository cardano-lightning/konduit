use crate::{cheque::Cheque, unlocked::Unlocked};
use anyhow::anyhow;
use cardano_tx_builder::PlutusData;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MixedCheque {
    Unlocked(Unlocked),
    Cheque(Cheque),
}

impl PartialOrd for MixedCheque {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.index().cmp(&other.index()) {
            std::cmp::Ordering::Less => Some(std::cmp::Ordering::Less),
            std::cmp::Ordering::Equal => None,
            std::cmp::Ordering::Greater => Some(std::cmp::Ordering::Greater),
        }
    }
}

impl Ord for MixedCheque {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.partial_cmp(other) {
            Some(ord) => ord,
            None => match (self, other) {
                (Self::Unlocked(a), Self::Unlocked(b)) => a.cmp(b),
                (Self::Cheque(a), Self::Cheque(b)) => a.cmp(b),
                (Self::Unlocked(_), _) => std::cmp::Ordering::Greater,
                (Self::Cheque(_), _) => std::cmp::Ordering::Less,
            },
        }
    }
}

impl MixedCheque {
    pub fn is_cheque(&self) -> bool {
        match self {
            &Self::Unlocked(_) => false,
            _ => true,
        }
    }

    pub fn index(&self) -> u64 {
        match self {
            Self::Unlocked(unlocked) => unlocked.cheque_body.index,
            Self::Cheque(cheque) => cheque.cheque_body.index,
        }
    }

    pub fn as_unlocked(&self) -> Option<Unlocked> {
        match self {
            MixedCheque::Unlocked(unlocked) => Some(unlocked.clone()),
            MixedCheque::Cheque(_) => None,
        }
    }

    pub fn as_cheque(&self) -> Option<Cheque> {
        match self {
            MixedCheque::Unlocked(_) => None,
            MixedCheque::Cheque(cheque) => Some(cheque.clone()),
        }
    }
}

impl From<Unlocked> for MixedCheque {
    fn from(value: Unlocked) -> Self {
        MixedCheque::Unlocked(value)
    }
}

impl From<Cheque> for MixedCheque {
    fn from(value: Cheque) -> Self {
        MixedCheque::Cheque(value)
    }
}

impl<'a> TryFrom<PlutusData<'a>> for MixedCheque {
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

        fn try_unlocked(fields: Vec<PlutusData<'_>>) -> anyhow::Result<MixedCheque> {
            Ok(MixedCheque::from(Unlocked::try_from(fields)?))
        }

        fn try_cheque(fields: Vec<PlutusData<'_>>) -> anyhow::Result<MixedCheque> {
            Ok(MixedCheque::from(Cheque::try_from(fields)?))
        }
    }
}

impl<'a> From<MixedCheque> for PlutusData<'a> {
    fn from(value: MixedCheque) -> Self {
        match value {
            MixedCheque::Unlocked(unlocked) => PlutusData::constr(0, Vec::from(unlocked)),
            MixedCheque::Cheque(cheque) => PlutusData::constr(1, Vec::from(cheque)),
        }
    }
}
