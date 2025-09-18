use aiken/builtin.{
  append_bytearray, head_list, sha2_256, tail_list, verify_ed25519_signature,
}
use aiken/cbor
use aiken/collection/list
use aiken/crypto.{Signature, VerificationKey}
use extra/xlist.{drop_hard, foldl4, foldl6}
use extra/xoption
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
  key: VerificationKey,
  tag: t.Tag,
  squash: t.Squash,
  unlockeds: List<t.Unlocked>,
  return: fn(Amount, Option<Timestamp>) -> result,
) -> result {
  let verify_signature =
    mk_verify_signature(key, tag, _, _)
  let (squash_body, squash_signature) = squash
  expect verify_signature(squash_body, squash_signature)
  let (amount, index, exclude) = squash_body
  let with =
    with_unlocked(verify_signature, _, _, _, _, _, _)
  let upper_bound, amount, _index, _exclude <-
    foldl4(unlockeds, None, amount, index, exclude, with, _)
  return(amount, upper_bound)
}

fn with_unlocked(
  verify_signature: fn(Data, Signature) -> Bool,
  unlocked: t.Unlocked,
  upper_bound: Option<Timestamp>,
  total: Amount,
  index: Index,
  exclude: t.Exclude,
  return: fn(Option<Timestamp>, Amount, Index, t.Exclude) -> result,
) -> result {
  let (body, signature, secret) = unlocked
  expect verify_signature(body, signature)
  let (new_index, amount, timeout, image) = body
  expect sha2_256(secret) == image
  let upper_bound = xoption.min(upper_bound, timeout)
  let total = total + amount
  let index, exclude <- next_index(index, exclude, new_index)
  return(upper_bound, total, index, exclude)
}

fn next_index(
  prev: Index,
  exclude: t.Exclude,
  curr: Index,
  return: fn(Index, t.Exclude) -> result,
) -> result {
  if prev < curr {
    return(curr, [])
  } else {
    return(prev, drop_hard(exclude, curr))
  }
}

pub fn verify_m_receipt(
  key: VerificationKey,
  tag: t.Tag,
  squash: t.Squash,
  mixs: List<t.Mix>,
  pends_amount: Amount,
  pends: List<t.PendCheque>,
  return: fn(Amount, Option<Timestamp>) -> result,
) -> result {
  let verify_signature =
    mk_verify_signature(key, tag, _, _)
  let (squash_body, squash_signature) = squash
  expect verify_signature(squash_body, squash_signature)
  let (amount, index, exclude) = squash_body
  let with =
    with_mix(verify_signature, _, _, _, _, _, _, _, _)
  let upper_bound, amount, _index, _exclude, pends_amount, pends <-
    foldl6(mixs, None, amount, index, exclude, pends_amount, pends, with, _)
  expect pends_amount == 0
  expect pends == []
  return(amount, upper_bound)
}

type ReturnWithMix<result> =
  fn(Option<Timestamp>, Amount, Index, t.Exclude, Amount, List<t.PendCheque>) ->
    result

pub fn with_mix(
  verify_signature: fn(Data, Signature) -> Bool,
  mix: t.Mix,
  upper_bound: Option<Timestamp>,
  total: Amount,
  index: Index,
  exclude: t.Exclude,
  pends_amount: Amount,
  pends: List<t.PendCheque>,
  return: ReturnWithMix<result>,
) -> result {
  when mix is {
    t.MUnlocked(body, signature, secret) -> {
      expect verify_signature(body, signature)
      let (new_index, amount, timeout, image) = body
      expect sha2_256(secret) == image
      let upper_bound = xoption.min(upper_bound, timeout)
      let total = total + amount
      let index, exclude <- next_index(index, exclude, new_index)
      return(upper_bound, total, index, exclude, pends_amount, pends)
    }
    t.MPend(body, signature) -> {
      expect verify_signature(body, signature)
      let (new_index, amount, timeout, image) = body
      let index, exclude <- next_index(index, exclude, new_index)
      expect [pend, ..pends] = pends
      expect pend == (amount, timeout, image)
      let pends_amount = pends_amount - amount
      return(upper_bound, total, index, exclude, pends_amount, pends)
    }
  }
}

pub fn verify_unlock(
  unpends: List<t.Unpend>,
  pends_in: List<t.PendCheque>,
  pends_out: List<t.PendCheque>,
  return: fn(Amount, Option<Timestamp>) -> result,
) -> result {
  let timeout, amount, pends_in, pends_out <-
    foldl4(unpends, None, 0, pends_in, pends_out, with_unpend_unlock, _)
  expect pends_in == pends_out
  return(amount, timeout)
}

type ReturnWithUnpendUnlock<result> =
  fn(Option<Timestamp>, Amount, List<t.PendCheque>, List<t.PendCheque>) ->
    result

pub fn with_unpend_unlock(
  unpend: t.Unpend,
  upper_bound: Option<Timestamp>,
  total: Amount,
  pends_in: List<t.PendCheque>,
  pends_out: List<t.PendCheque>,
  return: ReturnWithUnpendUnlock<result>,
) -> result {
  when unpend is {
    #"" -> {
      // Continue
      expect head_list(pends_in) == head_list(pends_out)
      return(upper_bound, total, tail_list(pends_in), tail_list(pends_out))
    }
    _ -> {
      // Unlock
      let (amount, timeout, hash) = head_list(pends_in)
      let upper_bound = xoption.min(upper_bound, timeout)
      expect sha2_256(unpend) == hash
      return(upper_bound, total + amount, tail_list(pends_in), pends_out)
    }
  }
}

pub fn verify_expire(
  unpends: List<t.Unpend>,
  pends_in: List<t.PendCheque>,
  pends_out: List<t.PendCheque>,
  return: fn(Amount, Option<Timestamp>) -> result,
) {
  let lower_bound, amount, pends_in, pends_out <-
    foldl4(unpends, None, 0, pends_in, pends_out, with_unpend_expire, _)
  expect pends_in == pends_out
  return(amount, lower_bound)
}

pub fn with_unpend_expire(
  unpend: t.Unpend,
  lower_bound: Option<Timestamp>,
  total: Amount,
  pends_in: List<t.PendCheque>,
  pends_out: List<t.PendCheque>,
  return: fn(Option<Timestamp>, Amount, List<t.PendCheque>, List<t.PendCheque>) ->
    result,
) -> result {
  when unpend is {
    #"" -> {
      // Continue
      expect head_list(pends_in) == head_list(pends_out)
      return(lower_bound, total, tail_list(pends_in), tail_list(pends_out))
    }
    _ -> {
      // Expire
      let (amount, timeout, _) = head_list(pends_in)
      let lower_bound = xoption.min(lower_bound, timeout)
      return(lower_bound, total + amount, tail_list(pends_in), pends_out)
    }
  }
}

pub fn verify_end(
  pends_in: List<t.PendCheque>,
  return: fn(Option<Timestamp>) -> result,
) -> result {
    // FIXME :: IS THIS MIN OR MAX
  pends_in
    |> list.foldl(None, fn((_, timeout, _), c) { xoption.max(c, timeout) })
    |> return
}
