use aiken/cbor
use aiken/crypto.{Signature, VerificationKey, sha2_256}
use aiken/primitive/bytearray
use env.{verify_signature}
use konduit/prelude.{Amount, Index, Timestamp}
use konduit/types as t

pub fn wellsigned(
  key: VerificationKey,
  tag: t.Tag,
  body: Data,
  signature: Signature,
) {
  verify_signature(key, cbor.serialise((tag, body)), signature)
}

pub fn unlocked(
  wellsinged: fn(Data, Signature) -> Bool,
  body: t.ChequeBody,
  signature: Signature,
  return: fn(Index, Amount, Timestamp) -> result,
) -> result {
  trace @"unlocked.1": @"the secret hashes to the latch"
  let (index, amount, timeout, secret) = body
  let lock = sha2_256(secret)
  trace @"unlocked.0": @"`(body, sig)` is well-signed"
  expect wellsinged((index, amount, timeout, lock), signature)
  trace @"unlocked.2": @"the secret length is 32"
  expect bytearray.length(secret) == 32
  return(index, amount, timeout)
}
