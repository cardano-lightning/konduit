use crate::BoxFuture;

/// Why a configured asset conversion could not be performed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// No usable rate data is currently available.
    #[error("conversion rate unavailable: {0}")]
    Unavailable(String),
    /// The rate or requested amount cannot produce a valid `u64` result.
    #[error("invalid conversion: {0}")]
    Invalid(String),
}

/// Converts between one Konduit inbound asset and a rail's outbound asset.
///
/// The asset pair is fixed by the FX configuration. Since both
/// directions return integer amounts, a round trip is approximate and may
/// differ from its starting amount by rounding. Both directions are
/// fallible: for example, rate data may be unavailable or a converted value
/// may not fit in `u64`.
pub trait Fx: Send + Sync {
    /// Convert inbound units to the rail's units using integer arithmetic.
    /// Implementations truncate fractional destination units; callers may
    /// adjust by one unit when their policy requires conservative coverage.
    fn inbound_to_outbound<'a>(&'a self, amount: u64) -> BoxFuture<'a, Result<u64, Error>>;

    /// Convert rail units to inbound units using integer arithmetic.
    /// Implementations truncate fractional destination units; callers may
    /// adjust by one unit when their policy requires a conservative fee.
    fn outbound_to_inbound<'a>(&'a self, amount: u64) -> BoxFuture<'a, Result<u64, Error>>;
}
