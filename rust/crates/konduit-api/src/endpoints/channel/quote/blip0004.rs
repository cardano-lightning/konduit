//! AMP — Atomic Multipath Payments (BLIP-0004).
//!
//! The server splits the payment across multiple paths atomically.
//! No invoice required; pre-image is derived from the cheque lock.
//!
//! TODO: define `Request` and implement.

pub type Response = crate::common::channel::quote::Response;

pub type Error = crate::common::channel::quote::Error;
