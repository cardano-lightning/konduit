use aiken/builtin.{
  append_bytearray, sha2_256, verify_ed25519_signature,
}
use aiken/cbor
use aiken/crypto.{Signature, VerificationKey}
use aiken/primitive/bytearray
use konduit/prelude.{Amount, Index, Timestamp}
use konduit/types as t

pub fn wellsigned(
  key: VerificationKey,
  tag: t.Tag,
  body: Data,
  signature: Signature,
) {
  verify_ed25519_signature(
    key,
    append_bytearray(tag, cbor.serialise(body)),
    signature,
  )
}

pub fn unlocked(
  wellsinged: fn(Data, Signature) -> Bool,
  body: t.ChequeBody,
  signature: Signature,
  secret: t.Secret,
  return: fn(Index, Amount, Timestamp) -> result,
) -> result {
  trace @"unlocked.0": @"`(body, sig)` is well-signed"
  expect wellsinged(body, signature)
  let (index, amount, timeout, lock) = body
  trace @"unlocked.1": @"the secret hashes to the lock"
  expect sha2_256(secret) == lock
  trace @"unlocked.2": @"the secret length is 32"
  expect bytearray.length(secret) == 32
  return(index, amount, timeout)
}
