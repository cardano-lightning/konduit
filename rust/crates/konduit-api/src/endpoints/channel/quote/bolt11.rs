//! BOLT11 : Invoice
use bln_sdk::types::Invoice;
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
#[serde(transparent)]
#[cbor(transparent)]
pub struct Request(
    #[cbor(n(0), with = "cbor_with::display_from_str")]
    #[serde_as(as = "DisplayFromStr")]
    Invoice,
);

pub type Response = crate::common::channel::quote::Response;

pub type Error = crate::common::channel::quote::Error;
