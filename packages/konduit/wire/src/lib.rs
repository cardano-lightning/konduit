//! # Konduit wire
//!
//! Interface between servers and clients
//!
//! ## Conventions
//!
//! The repo has conventions ultimately enshrined in `main.rs` that generates an API.md doc.
pub mod auth;
pub mod info;
pub mod limit;
pub mod reg;
pub mod version;

/// Used to indicate resource name
pub const ENDPOINT: &str = "";

/// Used to indicate resource path
pub const PATH: &str = "";
