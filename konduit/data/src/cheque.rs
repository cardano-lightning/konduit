use crate::{
    ChequeBody, Duration, Lock, Tag, Unverified, Verified, VerifyState, locked::Locked,
    unlocked::Unlocked,
};
use anyhow::anyhow;
use cardano_sdk::{PlutusData, VerificationKey};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cheque<V: VerifyState = Unverified> {
    Unlocked(Unlocked<V>),
    Locked(Locked<V>),
}

impl<V: VerifyState + Ord> PartialOrd for Cheque<V> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<V: VerifyState + Ord> Ord for Cheque<V> {
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

impl<V: VerifyState> Cheque<V> {
    pub fn is_cheque(&self) -> bool {
        !matches!(self, &Self::Unlocked(_))
    }

    pub fn body(&self) -> ChequeBody {
        match self {
            Self::Unlocked(unlocked) => unlocked.locked().body().clone(),
            Self::Locked(locked) => locked.body().clone(),
        }
    }

    pub fn index(&self) -> u64 {
        self.body().index()
    }

    pub fn amount(&self) -> u64 {
        self.body().amount()
    }

    pub fn lock(&self) -> Lock {
        *self.body().lock()
    }

    pub fn timeout(&self) -> Duration {
        self.body().timeout()
    }
}

impl<V: VerifyState> Cheque<V> {
    pub fn as_unlocked(&self) -> Option<Unlocked<V>> {
        match self {
            Cheque::Unlocked(unlocked) => Some(unlocked.clone()),
            Cheque::Locked(_) => None,
        }
    }

    pub fn as_locked(&self) -> Option<Locked<V>> {
        match self {
            Cheque::Unlocked(_) => None,
            Cheque::Locked(locked) => Some(locked.clone()),
        }
    }
}

// =========================================================================
// Unverified State Methods
// =========================================================================
impl Cheque<Unverified> {
    /// Verifies the cryptographic signature against the verifying key and tag.
    /// On success, consumes the unverified cheque and transitions it to `Cheque<Verified>`.
    pub fn try_verify(
        self,
        verification_key: &VerificationKey,
        tag: &Tag,
    ) -> anyhow::Result<Cheque<Verified>> {
        match self {
            Cheque::Unlocked(x) => x.try_verify(verification_key, tag).map(Cheque::from),
            Cheque::Locked(x) => x.try_verify(verification_key, tag).map(Cheque::from),
        }
    }
}

impl<V: VerifyState> From<Unlocked<V>> for Cheque<V> {
    fn from(value: Unlocked<V>) -> Self {
        Cheque::Unlocked(value)
    }
}

impl<V: VerifyState> From<Locked<V>> for Cheque<V> {
    fn from(value: Locked<V>) -> Self {
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

#[cfg(feature = "proptest")]
impl proptest::arbitrary::Arbitrary for Cheque {
    type Parameters = ();
    type Strategy = proptest::strategy::BoxedStrategy<Self>;
    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        prop_oneof![
            any::<crate::Unlocked>().prop_map(Cheque::Unlocked),
            any::<crate::Locked>().prop_map(Cheque::Locked),
        ]
        .boxed()
    }
}

#[cfg(feature = "cddl")]
impl cuddly::ToCddl for Cheque {
    fn cddl_ref() -> String {
        "cheque".to_string()
    }
    fn cddl_definition() -> Option<String> {
        // Plutus constructors are CBOR-tagged: alt 0 → tag 121, alt 1 → tag 122.
        Some(format!(
            "cheque = #6.121({}) / #6.122({})",
            crate::Unlocked::cddl_ref(),
            crate::Locked::cddl_ref(),
        ))
    }
}
