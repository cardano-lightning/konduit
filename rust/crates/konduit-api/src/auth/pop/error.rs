use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode, thiserror::Error)]
#[cfg_attr(feature = "cddl", derive(konduit_cddl::ToCddl))]
#[cfg_attr(feature = "cddl", cddl(name = "auth-pop-error"))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum Error {
    #[n(0)]
    #[error("missing Konduit-Keytag header")]
    MissingKeytag,
    #[n(1)]
    #[error("missing Konduit-Signature header")]
    MissingSignature,
    #[n(2)]
    #[error("malformed keytag")]
    BadKeytag,
    #[n(3)]
    #[error("malformed signature")]
    BadSignature,
}

impl crate::ApiError for Error {
    fn status_code(&self) -> u16 {
        401
    }
}
