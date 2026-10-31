use crate::Payme;
use konduit_data::{ChequeBody, Signature};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Encode, Decode)]
pub struct PayBody {
    #[n(0)]
    pub cheque_body: ChequeBody,
    #[n(1)]
    pub signature: Signature,
    #[n(2)]
    pub payme: Payme,
}
