// Originally based on pallas:::crypto
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[repr(transparent)]
pub struct Signature([u8; 64]);

impl Signature {
    pub fn new(bytes: [u8; 64]) -> Self {
        Self(bytes)
    }

    pub fn bytes(self) -> [u8; 64] {
        self.0
    }
}

impl From<[u8; 64]> for Signature {
    fn from(value: [u8; 64]) -> Self {
        Self::new(value)
    }
}

impl From<Signature> for [u8; 64] {
    fn from(value: Signature) -> Self {
        value.bytes()
    }
}
