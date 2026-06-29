use crate::channel::bucket;

use super::receipt;
use konduit_data::VerifyError;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("no backing: channel not funded on-chain")]
    Backing,
    #[error("no receipt: submit a null squash first")]
    NoReceipt,
    #[error("insufficient capacity: too many unresolved payments")]
    Capacity,
    #[error("insufficient funds")]
    Funds,
    #[error("limit: {0}")]
    Limit(#[from] bucket::Error),
    #[error("bad input")]
    Input,
    #[error("verify failed")]
    Verify,
    #[error("receipt {0}")]
    Receipt(#[from] receipt::Error),
}

impl From<VerifyError> for Error {
    fn from(_value: VerifyError) -> Self {
        Self::Verify
    }
}
