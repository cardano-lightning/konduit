//! Channel state management for Konduit L2 payment channels.
//!
//! The server is a dumb blob store: read blob → decode → apply event → encode → write.
//! This crate owns all events that mutate channel state.

pub mod backing;
pub use backing::{Backing, DepthBucket};

pub mod nota;
pub use nota::Nota;

mod channel;
pub use channel::Channel;

mod error;
pub use error::{Error, Result};
