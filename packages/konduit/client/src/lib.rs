//! # Konduit client
//!
//! Manages L1, and L2s for a wallet and add_vkey.
//!
//! Limitations:
//!
//! - one fuel wallet and one L1.
//! - one signer (`add_vkey`)
//! - tags are unique (for the key)
//! - a commit for a lock is a commit on the triple `(lock, tag, index)`.
//!   Retries on a lock are possible, but only on the same tag and index,
//!   and only once the previous attempt is declared `Ko`.
//! - limited delegation, but covers enough to set a delegation and
//!   change the delegation credential.

mod prelude;
pub(crate) use prelude::*;

mod keys;

mod commitments;
pub use commitments::{Commitment, Commitments};

pub mod cardano;
pub use cardano::Cardano;

pub mod l1;
pub use l1::L1;

pub mod server;

pub mod l2;
pub use l2::L2;

mod config;
pub use config::Config;

mod cache;
pub use cache::Cache;

#[cfg(feature = "cli")]
pub mod cli;

mod time;
mod utxo_batch;
