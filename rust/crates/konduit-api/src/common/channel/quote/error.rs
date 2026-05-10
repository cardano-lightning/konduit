use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode, thiserror::Error)]
pub enum Error {
    /// Request type not supported
    /// For example, Adaptor will not accept Simple quotes,
    /// and Consumer must send entire invoice.
    #[n(0)]
    #[error("Unsuported")]
    Unsuported,
    /// Will not accept payloads over 1024 Bytes
    #[n(1)]
    #[error("Too large")]
    Size,
    /// Limit
    #[n(2)]
    #[error("Limit exceeded")]
    Limit(#[n(2)] String),
    /// Other Error
    #[n(3)]
    #[error("Other: {0}")]
    Other(#[n(3)] String),
}
