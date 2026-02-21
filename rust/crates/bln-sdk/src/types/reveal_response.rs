#[derive(Debug, Clone)]
pub struct RevealResponse {
    pub secret: Option<[u8; 32]>,
}
