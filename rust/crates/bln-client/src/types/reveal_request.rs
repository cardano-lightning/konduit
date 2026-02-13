#[derive(Debug, Clone)]
pub struct RevealRequest {
    pub lock: [u8; 32],
}
