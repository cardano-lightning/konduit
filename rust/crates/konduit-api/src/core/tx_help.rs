use cardano_sdk::{Address, Hash, address::kind::Shelley};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct TxHelp {
    #[cbor(n(0), with = "crate::cbor_with::display_from_str")]
    #[serde_as(as = "serde_with::DisplayFromStr")]
    pub host_address: Address<Shelley>,
    #[n(1)]
    #[serde_as(as = "serde_with::hex::Hex")]
    pub validator: Hash<28>,
}
