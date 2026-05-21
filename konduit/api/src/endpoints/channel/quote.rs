//! `/channel/quote/*`
//!
//! All quote endpoints share `cheque-proposal` as their response type
//! and `quote-error` as their error type.

pub mod blip0003;
pub mod blip0004;
pub mod bln_template;
pub mod bolt11;
pub mod cl_template;
