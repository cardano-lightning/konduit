// Protocol Constants

pub const MAX_UNSQUASHED: usize = 10;

/// FIXME :: This should not be enforced here.
/// Consumer can safely construct "unsafe" squashes under sensible conditions.
/// Only Adaptor needs to enforce this. They should do so downstream.
pub const MAX_EXCLUDE_LENGTH: usize = 10;

// On-chain datatypes

mod cheque;
pub use cheque::Cheque;

mod cheque_body;
pub use cheque_body::ChequeBody;

mod cheque_signed;
pub use cheque_signed::ChequeSigned;

mod constants;
pub use constants::Constants;

mod crypto;
pub use crypto::{Signature, SigningKey, VerifyingKey};

mod datum;
pub use datum::Datum;

mod duration;
pub use duration::Duration;

mod indexes;
pub use indexes::{Indexes, IndexesError};

mod lock;
pub use lock::Lock;

mod locked;
pub use locked::Locked;

mod pending;
pub use pending::Pending;

mod redeemer;
pub use redeemer::{Cont, Eol, Redeemer, Step};

mod secret;
pub use secret::Secret;

mod squash;
pub use squash::Squash;

mod squash_body;
pub use squash_body::{SquashBody, SquashBodyError};

mod stage;
pub use stage::Stage;

mod tag;
pub use tag::Tag;

mod unlocked;
pub use unlocked::Unlocked;

mod unpend;
pub use unpend::Unpend;

mod used;
pub use used::Used;

// Other

mod verify_error;
pub use verify_error::VerifyError;

mod verify_state;
pub use verify_state::{Unverified, Verified, VerifyState};

pub(crate) mod utils;

mod parse_error;
pub use parse_error::ParseError;

pub mod cbor_with;
