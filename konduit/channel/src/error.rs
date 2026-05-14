#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("no backing: channel not funded on-chain")]
    Backing,
    #[error("no receipt: submit a null squash first")]
    Receipt,
    #[error("insufficient capacity: too many unresolved payments")]
    Capacity,
    #[error("insufficient funds")]
    Funds,
    #[error("bad input")]
    Input,
}

pub type Result<T> = std::result::Result<T, Error>;
