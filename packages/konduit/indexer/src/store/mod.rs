//! Storage backends for the konduit indexer.
//!
//! The public surface is intentionally minimal for now: a [`Store`] trait in
//! the crate root and a single SQLite implementation behind
//! [`sqlite::SqliteStore`]. Future backends (e.g. Postgres) can be added
//! here without disturbing the rest of the crate.

pub mod sqlite;
