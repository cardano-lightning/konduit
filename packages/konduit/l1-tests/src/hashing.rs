use cryptoxide::blake2b::Blake2b;
use cryptoxide::digest::Digest;

pub fn hash32(input: &[u8]) -> [u8; 32] {
    let mut hasher = Blake2b::new(32); // 32 = output size in bytes
    hasher.input(input);
    let mut out = [0u8; 32];
    hasher.result(&mut out);
    out
}
