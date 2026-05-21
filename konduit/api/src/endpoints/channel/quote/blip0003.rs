//! Keysend (BLIP-0003): spontaneous payment without an invoice.
//!
//! The server routes a keysend payment directly to the payee's node.
//! No invoice is required — the pre-image is derived from the cheque lock.
//!
//! TODO: define `Request` and implement.

pub type Response = crate::common::channel::quote::Response;

pub type Error = crate::common::channel::quote::Error;
