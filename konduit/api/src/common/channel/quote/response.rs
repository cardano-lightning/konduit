use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

/// The server's proposal for a new cheque.
///
/// Returned by all `POST /channel/quote/*` endpoints.
/// The client uses these values to construct and sign a [`Cheque`][konduit_data::Cheque]
/// which is then submitted via `POST /channel/pay/quoted`.
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
#[cfg_attr(feature = "cddl", derive(konduit_cddl::ToCddl))]
#[cfg_attr(feature = "cddl", cddl(name = "cheque-proposal"))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Response {
    /// Cheque index
    #[n(0)]
    pub index: u64,
    /// Cheque amount
    #[n(1)]
    pub amount: u64,
    /// Cheque timeout. Note that this is **relative** and in ms
    #[n(2)]
    pub relative_timeout: u64,
    /// Routing fee. Informational.
    /// Clients should independently calculate the effective fee
    #[n(3)]
    pub fee: u64,
}
