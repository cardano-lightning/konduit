use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub enum Error {
    /// Request type not supported
    /// For example, Adaptor will not accept Simple quotes,
    /// and Consumer must send entire invoice.
    #[n(0)]
    Unsuported,
    /// Will not accept payloads over 1024 Bytes
    #[n(1)]
    Size,
    /// Will not accept payloads over 1024 Bytes
    #[n(2)]
    Other(#[n(0)] String),
}
