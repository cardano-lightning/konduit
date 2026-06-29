//! CLN template: a bare bones pay request for Cardano Lightning.

use konduit_data::Lock;
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

pub const ENDPOINT: &str = "/quote";
pub const PATH: &str = const_format::concatcp!(super::PATH, ENDPOINT);

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct Body {
    /// Payee: their ed25519 verifying key
    #[n(0)]
    #[serde_as(as = "serde_with::hex::Hex")]
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
