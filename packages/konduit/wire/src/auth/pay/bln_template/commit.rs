use konduit_data::Locked;
use minicbor::{Decode, Encode};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub const ENDPOINT: &str = "/commit";
pub const PATH: &str = const_format::concatcp!(super::PATH, ENDPOINT);

#[derive(Debug, Clone, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Body {
    #[n(0)]
    pub cheque: Locked,
    #[n(1)]
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            skip_serializing_if = "Option::is_none",
            with = "serde_with::As::<Option<serde_with::hex::Hex>>"
        )
    )]
    pub payment_secret: Option<[u8; 32]>,
}

pub type Response = crate::auth::pay::common::quote::ChequeProposal;

pub type Error = crate::auth::Error<DomainError>;

pub type DomainError = crate::auth::pay::common::commit::Error;
