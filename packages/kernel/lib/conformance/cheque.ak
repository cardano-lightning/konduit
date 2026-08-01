use aiken/crypto.{VerificationKey, sha2_256}
use aiken/primitive/bytearray
use konduit/types as t
use konduit/wellformed as wf

pub type Verify {
  key: VerificationKey,
  tag: t.Tag,
  cheque: t.Cheque,
}

pub fn verify(x: Verify) -> Bool {
  let (body, signature) = when x.cheque is {
    t.UnlockedCheque(body, signature) -> {
      let (index, amount, timeout, secret) = body
      expect bytearray.length(secret) == 32
      let lock = sha2_256(secret)
      ((index, amount, timeout, lock), signature)
    }
    t.LockedCheque(body, sig) -> (body, sig)
  }
  expect wf.wellsigned(x.key, x.tag, body, signature)
  True
}
