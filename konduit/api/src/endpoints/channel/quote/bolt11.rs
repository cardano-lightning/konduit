//! BOLT-11 : Pay a Lightning invoice.
use bln_sdk::types::Invoice;
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
#[serde(transparent)]
#[cbor(transparent)]
#[cfg_attr(feature = "cddl", derive(konduit_cddl::ToCddl))]
#[cfg_attr(feature = "cddl", cddl(name = "quote-bolt11-request", inner = "text"))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Request(
    #[cbor(n(0), with = "cbor_with::display_from_str")]
    #[serde_as(as = "DisplayFromStr")]
    pub Invoice,
);

pub type Response = crate::common::channel::quote::Response;

pub type Error = crate::common::channel::quote::Error;
