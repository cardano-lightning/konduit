use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct Request {}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct Response {}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub enum Error {}
