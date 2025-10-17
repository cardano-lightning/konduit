use blake2::{
    Blake2b,
    Digest,
    digest::consts::U28, // The output size: 224 bits = 28 bytes
};

pub fn blake2b_224(data: &[u8]) -> [u8; 28] {
    let mut hasher = Blake2b::<U28>::new();
    hasher.update(data);
    hasher.finalize().into()
}
