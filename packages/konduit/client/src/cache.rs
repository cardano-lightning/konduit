//! A basic cache to manage l1(s) and l2s.
//! Allows for a stateful sequence.
//!
//! Limitations:
//!
//! - one fuel wallet and one L1.
//! - one signer (`add_vkey`)
//! - tags are unique (for the key)
//! - A commit for a lock is a commit on the triple `(lock, tag, index)`.
//! Retries on a lock are possible, but only on the same tag and index.
//! Moreover, retries are possible only when the prev attempt is declared `Ko`.
//! - limited delegation, but covers enough to set a delegation and change the delgation credential.

use std::collections::BTreeMap;

use konduit_data::{Lock, Tag};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::{l1, l2};

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct Cache {
    /// Set when running an embedded wallet, else managed externally.
    #[n(0)]
    wallet_key: Option<[u8; 32]>,
    /// Set when running an embedded signer, else managed externally.
    #[n(1)]
    add_vkey: Option<[u8; 32]>,
    #[n(2)]
    l1: l1::Cache,
    #[n(3)]
    l2s: BTreeMap<Tag, l2::Cache>,
    /// Commits a lock to a Tag,
    #[n(4)]
    commits: BTreeMap<Lock, Tag>,
}
