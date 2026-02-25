use crate::ChannelParameters;
use cardano_sdk::{Address, Hash, address::kind::Shelley};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptorInfo {
    // Terms of service. Purely informational
    pub tos: TosInfo,
    // Channel parameters
    pub channel_parameters: ChannelParameters,
    // Tx building
    pub tx_help: TxHelp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TosInfo {
    pub flat_fee: u64,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxHelp {
    #[serde_as(as = "serde_with::DisplayFromStr")]
    pub host_address: Address<Shelley>,
    #[serde_as(as = "serde_with::hex::Hex")]
    pub validator: Hash<28>,
}

#[cfg(feature = "wasm")]
pub(crate) mod wasm {
    use cardano_sdk::VerificationKey;
    use serde::{Deserialize, Serialize};
    use std::ops::Deref;
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    #[repr(transparent)]
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct AdaptorInfo(super::AdaptorInfo);

    impl Deref for AdaptorInfo {
        type Target = super::AdaptorInfo;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    #[wasm_bindgen]
    impl AdaptorInfo {
        #[wasm_bindgen(getter, js_name = "verificationKey")]
        pub fn verification_key(&self) -> VerificationKey {
            self.channel_parameters.adaptor_key
        }

        #[wasm_bindgen(getter, js_name = "closePeriod")]
        pub fn close_period_secs(&self) -> u64 {
            self.channel_parameters.close_period.as_secs()
        }

        #[wasm_bindgen(getter, js_name = "maxTagLength")]
        pub fn max_tag_length(&self) -> u8 {
            self.channel_parameters.tag_length as u8
        }

        #[wasm_bindgen(getter, js_name = "fee")]
        pub fn fee(&self) -> u64 {
            self.tos.flat_fee
        }
    }
}
