use crate::Duration;
use cardano_sdk::VerificationKey;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

/// These variables are either those used by more than one component,
/// or are mandatory.
/// These are not all variables required: component specific ones
/// are colocated with the component.
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelParameters {
    #[serde_as(as = "serde_with::hex::Hex")]
    pub adaptor_key: VerificationKey,
    pub close_period: Duration,
    pub tag_length: usize,
}
