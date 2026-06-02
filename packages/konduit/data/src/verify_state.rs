use serde::{Deserialize, Serialize};

pub trait VerifyState: sealed::Sealed + Clone {}

mod sealed {
    pub trait Sealed {}
    impl Sealed for super::Unverified {}
    impl Sealed for super::Verified {}
}

/// Verification state marker for a cheque that has not yet been cryptographically verified.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Unverified;

/// Verification state marker for a cheque that has been cryptographically validated.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Verified;

impl VerifyState for Unverified {}
impl VerifyState for Verified {}
