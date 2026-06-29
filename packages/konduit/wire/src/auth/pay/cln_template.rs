pub mod commit;
pub mod quote;

const ENDPOINT: &str = "/cln_template";
pub const PATH: &str = const_format::concatcp!(super::PATH, ENDPOINT);
