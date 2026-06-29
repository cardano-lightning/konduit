use konduit_data::Locked;
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

pub const ENDPOINT: &str = "/commit";
pub const PATH: &str = const_format::concatcp!(super::PATH, ENDPOINT);

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct Body {
    #[n(0)]
    pub cheque: Locked,
    #[n(1)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde_as(as = "Option<serde_with::hex::Hex>")]
    pub payment_secret: Option<[u8; 32]>,
}

pub type Response = crate::auth::pay::common::quote::ChequeProposal;

pub type Error = crate::auth::Error<DomainError>;

pub type DomainError = crate::auth::pay::common::commit::Error;
