use konduit_data::Cheque;
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
#[serde(transparent)]
#[cbor(transparent)]
pub struct Request {
    #[cbor(n(0), with = "konduit_data::cbor_with::plutus_data")]
    cheque: Cheque,
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub enum Response {
    #[n(0)]
    Inflight,
    #[n(1)]
    Ok,
    #[n(2)]
    Ko,
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub enum Error {}
