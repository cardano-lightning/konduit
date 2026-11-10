//! Shared consumer/provider L2 endpoint contracts.

mod adaptor_channel_parameters;
pub use adaptor_channel_parameters::AdaptorChannelParameters;

mod adaptor_info;

pub mod info {
    pub use crate::adaptor_info::{TosInfo, TxHelp};
    pub type Response<T = TxHelp> = crate::adaptor_info::AdaptorInfo<T>;
    pub type Error = crate::Error;
    pub type Result<T = Response> = std::result::Result<T, Error>;
}

pub mod pay;
pub mod quote;

mod squash_status;

pub mod receipt {
    pub type Response = Option<konduit_core::WireReceipt>;
    pub type Error = konduit_core::receipt::Error;
    pub type Result = std::result::Result<Response, Error>;
}

pub mod squash {
    pub use crate::squash_status::SquashStatus as Response;
    pub use konduit_core::SquashProposal as Proposal;
    pub type Request = konduit_core::Squash;
    pub type Error = crate::Error;
    pub type Result = std::result::Result<Response, Error>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "cbor", derive(minicbor::Encode, minicbor::Decode))]
pub struct Error {
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    #[cfg_attr(feature = "cbor", n(0))]
    pub slug: String,
    #[cfg_attr(feature = "cbor", n(1))]
    pub status: u16,
    #[cfg_attr(feature = "cbor", n(2))]
    pub detail: String,
}
