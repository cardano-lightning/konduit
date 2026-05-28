use aiken/crypto.{VerificationKey}

pub const min_ada_buffer = 2_000_000

pub const verify_signature =
  fn(_key: VerificationKey, _message: ByteArray, _signature: ByteArray) { True }

pub fn verify_hash(_in: ByteArray, _out: ByteArray) {
  True
}
