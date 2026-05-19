use serde::{Deserialize, Serialize};

/// Verification state marker for a cheque that has not yet been cryptographically verified.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Unverified;

/// Verification state marker for a cheque that has been cryptographically validated.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Verified;
