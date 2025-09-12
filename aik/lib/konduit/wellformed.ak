use aiken/builtin.{append_bytearray, sha2_256, verify_ed25519_signature}
use aiken/cbor
use aiken/crypto.{Signature, VerificationKey}
use extra/xlist.{drop_hard, foldl3, foldl5}
use konduit/prelude.{Amount, Index, Timestamp}
use konduit/types as t

pub fn mk_verify_signature(
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

pub fn verify_unlocked(
  upper_bound: Timestamp,
  key: VerificationKey,
  tag: t.Tag,
  unlocked: t.Unlocked,
) {
  let (body, signature, preimage) = unlocked
  let (_, _, timeout, image) = body
  and {
    mk_verify_signature(key, tag, body, signature)?,
    (upper_bound < timeout)?,
    (sha2_256(preimage) == image)?,
  }
}

pub fn verify_receipt(
  upper_bound: Timestamp,
  key: VerificationKey,
  tag: t.Tag,
  squash: t.Squash,
  unlockeds: List<t.Unlocked>,
) {
  let verify_signature =
    mk_verify_signature(key, tag, _, _)
  let (squash_body, squash_signature) = squash
  expect verify_signature(squash_body, squash_signature)
  let (amount, index, exclude) = squash_body
  let with =
    mk_with_unlocked(upper_bound, verify_signature, _, _, _, _, _)
  let amount, _index, _exclude <-
    foldl3(unlockeds, amount, index, exclude, with, _)
  amount
}

pub fn mk_with_unlocked(
  upper_bound: Timestamp,
  verify_signature: fn(Data, Signature) -> Bool,
  unlocked: t.Unlocked,
  amount: Amount,
  index: Index,
  exclude: t.Exclude,
  return: fn(Amount, Index, t.Exclude) -> Amount,
) {
  let (body, signature, secret) = unlocked
  expect verify_signature(body, signature)
  let (new_index, new_amount, timeout, image) = body
  expect upper_bound < timeout
  expect sha2_256(secret) == image
  if index < new_index {
    return(amount + new_amount, new_index, [])
  } else {
    return(amount + new_amount, index, drop_hard(exclude, new_index))
  }
}

pub fn verify_m_receipt(
  upper_bound: Timestamp,
  key: VerificationKey,
  tag: t.Tag,
  squash: t.Squash,
  mixs: List<t.Mix>,
  pends_amount: Amount,
  pends: List<t.PendCheque>,
) {
  let verify_signature =
    mk_verify_signature(key, tag, _, _)
  let (squash_body, squash_signature) = squash
  expect verify_signature(squash_body, squash_signature)
  let (amount, index, exclude) = squash_body
  let with =
    mk_with_mix(upper_bound, verify_signature, _, _, _, _, _, _, _)
  let amount, _index, _exclude, pends_amount, pends <-
    foldl5(mixs, amount, index, exclude, pends_amount, pends, with, _)
  expect pends_amount == 0
  expect pends == []
  amount
}

pub fn mk_with_mix(
  upper_bound: Timestamp,
  verify_signature: fn(Data, Signature) -> Bool,
  mix: t.Mix,
  amount: Amount,
  index: Index,
  exclude: t.Exclude,
  pends_amount: Amount,
  pends: List<t.PendCheque>,
  return: fn(Amount, Index, t.Exclude, Amount, List<t.PendCheque>) -> Amount,
) {
  when mix is {
    t.MUnlocked(body, signature, secret) -> {
      expect verify_signature(body, signature)
      let (new_index, new_amount, timeout, image) = body
      expect upper_bound < timeout
      expect sha2_256(secret) == image
      if index < new_index {
        return(amount + new_amount, new_index, exclude, pends_amount, pends)
      } else {
        return(
          amount + new_amount,
          index,
          drop_hard(exclude, new_index),
          pends_amount,
          pends,
        )
      }
    }
    t.MPend(body, signature) -> {
      expect verify_signature(body, signature)
      let (new_index, new_amount, timeout, image) = body
      // expect upper_bound < timeout
      // expect sha2_256(secret) == image
      expect [pend, ..pends] = pends
      expect pend == (new_amount, timeout, image)
      let pends_amount = pends_amount - new_amount
      if index < new_index {
        return(amount, new_index, [], pends_amount, pends)
      } else {
        return(
          amount,
          index,
          drop_hard(exclude, new_index),
          pends_amount,
          pends,
        )
      }
    }
  }
}

pub fn verify_secrets(
  upper_bound: Timestamp,
  tag: t.Tag,
  add_vkey: VerificationKey,
  pends_amount_in: Amount,
  pends_in: List<t.PendCheque>,
  pends_amount_out: Amount,
  pends_out: List<t.PendCheque>,
  secrets: List<(Index, t.Secret)>,
) {
  todo
}

pub fn verify_expire(
  lower_bound: Timestamp,
  tag: t.Tag,
  add_vkey: VerificationKey,
  pends_amount_in: Amount,
  pends_in: List<t.PendCheque>,
  pends_amount_out: Amount,
  pends_out: List<t.PendCheque>,
) {
  todo
}
