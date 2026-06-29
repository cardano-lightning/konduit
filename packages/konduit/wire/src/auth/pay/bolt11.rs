pub mod commit;
pub mod quote;

const ENDPOINT: &str = "/bolt11";
pub const PATH: &str = const_format::concatcp!(super::PATH, ENDPOINT);
