//! Payment schemes
pub mod bln_template;
pub mod bolt11;
pub mod cln_template;
pub mod common;

const ENDPOINT: &str = "/pay";
pub const PATH: &str = const_format::concatcp!(super::PATH, ENDPOINT);
