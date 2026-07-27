use konduit_data::{ChequeBody, Signature};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PayBody {
    pub cheque_body: ChequeBody,
    pub signature: Signature,
    pub invoice: String,
    // #[serde(with = "hex")]
    // pub payee: [u8; 33],
    // pub amount_msat: u64,
    // #[serde(with = "hex")]
    // pub payment_secret: [u8; 32],
    // pub final_cltv_delta: u64,
}
