//! A channel is what the adaptor recognizes as the channel:
//! All the state and logic associated to the adaptors needs.
//! This includes:
//!
//! + utxos at a kernel address backing the channel
//! + cheques and squashes
//! + state concerning the users off-chain usage such resource usage

use cardano_sdk::VerificationKey;
use konduit_data::{Receipt, Tag};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::backing::Backing;
use crate::nota::Nota;

#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct Channel {
    #[serde_as(as = "serde_with::hex::Hex")]
    #[cbor(n(0), with = "cbor_with::display_from_str")]
    key: VerificationKey,
    #[serde_as(as = "serde_with::hex::Hex")]
    #[cbor(n(1), with = "cbor_with::display_from_str")]
    tag: Tag,
    #[n(2)]
    backing: Backing,
    #[cbor(n(3), with = "cbor_with::nullable_same")]
    receipt: Option<Receipt>,
    #[n(4)]
    nota: Nota,
}
