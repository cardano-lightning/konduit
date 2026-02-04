use cardano_tx_builder::VerificationKey;
use konduit_data::Duration;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

mod args;
mod metavar;
pub use args::CommonArgs as Args;

/// These variables are either those used by more than one component,
/// or are mandatory.
/// These are not all variables required: component specific ones
/// are colocated with the component.
/// FIXME :: this is used by info and (kinda) admin.
/// I'm not sure where it belongs
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelParameters {
    #[serde_as(as = "serde_with::hex::Hex")]
    pub adaptor_key: VerificationKey,
    pub close_period: Duration,
    pub tag_length: usize,
}

impl ChannelParameters {
    pub fn from_args(args: Args) -> Self {
        let Args {
            signing_key,
            close_period,
            tag_length,
            ..
        } = args;
        let adaptor_key = VerificationKey::from(&signing_key);
        Self {
            adaptor_key,
            close_period,
            tag_length,
        }
    }
}
