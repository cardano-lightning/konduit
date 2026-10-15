use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct Quote {
    #[n(0)]
    pub index: u64,
    #[n(1)]
    pub amount: u64,
    #[n(2)]
    pub relative_timeout: u64,
    #[n(3)]
    pub routing_fee: u64,
    #[n(4)]
    pub invoice_hash: Option<String>,
    #[n(5)]
    pub invoice_amount_msat: u64,
    #[n(6)]
    pub payment_amount: u64,
    #[n(7)]
    pub routing_fee_amount: u64,
    #[n(8)]
    pub adaptor_fee: u64,
    #[n(9)]
    pub expires_at_epoch_millis: u64,
}
