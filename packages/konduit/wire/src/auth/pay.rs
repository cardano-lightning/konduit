//! The pay process is a sequence of steps.
//!
//! Generally the flow is as follows:
//!
//! - `quote`: the user requests a `quote` to determine whether a payment is possible,
//! and if so, the conditions of servicing the payment such as the amount, and timeout.
//! The quote is valid for a short amount of time, and is non-binding.
//! It may be rejected due to change in circumstances.
//!
//! - `commit`: the user commits to the payment. On the happy path, a commitment
//! is swiftly resolved, and the response is the corresponding secret.
//! Otherwise the associated funds cannot be otherwise spent (or committed).
//!
//! The pay flow follows a "scheme", which identifies with a particular spec.
//! For exampl, and most notably, `bolt11` aka Bitcoin Lightning invoice.
//! However, schemes are completely uncoupled. It is forseeable that some
//! servers offer only a subset of schemes, while some clients only support
//! a subset of schemes.
pub mod bln_template;
pub mod bolt11;
pub mod cln_template;
pub mod common;

const ENDPOINT: &str = "/pay";
pub const PATH: &str = const_format::concatcp!(super::PATH, ENDPOINT);
