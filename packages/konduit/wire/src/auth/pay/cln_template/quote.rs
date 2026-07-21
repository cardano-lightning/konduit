//! CLN template: a bare bones pay request for Cardano Lightning.

use konduit_data::Lock;
use minicbor::{Decode, Encode};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub const ENDPOINT: &str = "/quote";
pub const PATH: &str = const_format::concatcp!(super::PATH, ENDPOINT);

#[derive(Debug, Clone, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Body {
    /// Payee: their ed25519 verifying key
    #[n(0)]
    #[cfg_attr(
        feature = "serde",
        serde(with = "serde_with::As::<serde_with::hex::Hex>")
    )]
    pub payee: [u8; 32],
    /// Amount is in the unit currency.
    /// For example: lovelace for an Ada backed channel.
    #[n(1)]
    pub amount: u64,
    /// FIXME :: Move to `order`. The lock
    #[n(2)]
    pub lock: Lock,
}

pub type Response = crate::auth::pay::common::quote::ChequeProposal;

pub type Error = crate::auth::Error<DomainError>;

pub type DomainError = crate::auth::pay::common::quote::Error;
