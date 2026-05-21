use cardano_sdk::Signature;
use konduit_data::ChequeBody;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PayBody {
    pub cheque_body: ChequeBody,
    #[serde_as(as = "serde_with::hex::Hex")]
    pub signature: Signature,
    pub invoice: String,
}
