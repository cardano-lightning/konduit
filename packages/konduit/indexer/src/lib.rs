//! SQLite-backed indexer for the Konduit channel-state DB.
//!
//! This crate currently owns:
//! - the connection-management and schema-migration scaffolding
//!   ([`SqliteStore`]),
//! - the close-to-DB layer of typed record shapes and the [`Queries`] handle.

mod api;
mod error;

pub mod store;

pub use api::Store;
pub use error::{Error, Result};
pub use store::sqlite::SqliteStore;
pub use store::sqlite::queries::{
    BlockRecord, ChannelId, ChannelRecord, NewBlock, NewChannel, NewStepRecord, Queries,
    StepRecord, ThreadRecord,
};
