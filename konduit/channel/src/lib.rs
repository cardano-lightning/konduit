//! Channel state management for Konduit L2 payment channels.
//!
//! The server is a dumb blob store: read blob → decode → apply event → encode → write.
//! This crate owns all events that mutate channel state.

mod channel;
pub use channel::Channel;

mod error;
pub use error::{Error, Result};
