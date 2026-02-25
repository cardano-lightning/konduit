use crate::ChequeBody;
use cardano_sdk::Signature;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PayBody {
    pub cheque_body: ChequeBody,
    #[serde_as(as = "serde_with::hex::Hex")]
    pub signature: Signature,
    pub invoice: String,
    // #[serde(with = "hex")]
    // pub payee: [u8; 33],
    // pub amount_msat: u64,
    // #[serde(with = "hex")]
    // pub payment_secret: [u8; 32],
    // pub final_cltv_delta: u64,
}
