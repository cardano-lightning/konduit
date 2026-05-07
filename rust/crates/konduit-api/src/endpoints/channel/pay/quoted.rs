use konduit_data::Cheque;
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
#[serde(transparent)]
#[cbor(transparent)]
pub struct Request {
    #[cbor(n(0), with = "cbor_with::via_plutus_data")]
    cheque: Cheque,
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub enum Response {
    #[n(0)]
    Inflight,
    #[n(1)]
    Ok,
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub enum Error {}
