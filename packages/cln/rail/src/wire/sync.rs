use konduit_data::{Cheque, Squash};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

pub const PATH: &str = "/ch/x/cln/sync";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct Receipt {
    #[n(0)]
    pub squash: Squash,
    #[n(1)]
    pub cheques: Vec<Cheque>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct Request {
    #[n(0)]
    pub receipt: Receipt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct Response {
    #[n(0)]
    pub receipt: Receipt,
}
