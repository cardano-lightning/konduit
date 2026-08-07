use bln_sdk::types::Invoice;
use konduit_data::{ChequeBody, Signature};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone, Encode, Decode)]
pub struct PayBody {
    #[n(0)]
    pub cheque_body: ChequeBody,
    #[n(1)]
    pub signature: Signature,
    #[serde_as(as = "DisplayFromStr")]
    #[cbor(
        n(2),
        encode_with = "crate::cbor::encode_display",
        decode_with = "crate::cbor::decode_from_str"
    )]
    pub invoice: Invoice,
    // #[serde(with = "hex")]
    // pub payee: [u8; 33],
    // pub amount_msat: u64,
    // #[serde(with = "hex")]
    // pub payment_secret: [u8; 32],
    // pub final_cltv_delta: u64,
}
