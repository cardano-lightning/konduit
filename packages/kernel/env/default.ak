use aiken/crypto.{sha2_256, verify_ed25519_signature}

pub const min_ada_buffer = 2_000_000

pub const verify_signature = verify_ed25519_signature

pub fn verify_hash(in: ByteArray, out: ByteArray) {
  sha2_256(in) == out
}
