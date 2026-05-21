use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

/// Coarse confirmation depth classification reported to clients in [`BackingView`].
///
/// The server maps its internal `BlockDepth` to a bucket using its configured
/// finality thresholds — the bucket boundaries are server policy, not protocol.
/// Clients treat the bucket as opaque risk metadata for fee estimation.
///
/// [`BackingView`]: crate::endpoints::channel::sync::BackingView
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Encode, Decode,
)]
#[cfg_attr(feature = "cddl", derive(konduit_cddl::ToCddl))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum DepthBucket {
    /// Seen on-chain but very shallow — adversarially exploitable.
    #[n(0)]
    Unconfirmed,
    /// A few confirmations — rollback possible, elevated risk.
    #[n(1)]
    Shallow,
    /// Moderate confirmations — rollback unlikely but not negligible.
    #[n(2)]
    Probable,
    /// Deep enough to treat as practically final for fee purposes.
    #[n(3)]
    Deep,
    /// Beyond the finality window — floor is settled, zero exposure.
    #[n(4)]
    Settled,
}
