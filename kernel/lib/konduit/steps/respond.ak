use aiken/crypto.{VerificationKey}
use cardano/assets.{Value}
use extra/xlist.{foldl6}
use extra/xoption
use konduit/assets.{amount_in, amount_out} as k_assets
use konduit/prelude.{Amount, Index, Signature, Timestamp}
use konduit/steps/common.{Return, next_index}
use konduit/types as t
use konduit/wellformed as wf

pub fn do(
  constants: t.Constants,
  value_in: Value,
  stage_in: t.Stage,
  value_out: Value,
  stage_out: t.Stage,
  squash: t.Squash,
  mixeds: List<t.MixedCheque>,
  return: Return<result>,
) -> result {
  let t.Constants { tag, add_vkey, sub_vkey, .. } = constants
  expect t.Closed(subbed_in, _) = stage_in
  expect t.Responded(pends_amount, pends) = stage_out
  let subbed_out = subbed_in - amount_in(value_in) + amount_out(value_out)
  let amount, upper_bound <-
    account(add_vkey, tag, squash, mixeds, pends_amount, pends, _)
  expect amount >= subbed_out
  return(sub_vkey, None, upper_bound)
}

fn account(
  key: VerificationKey,
  tag: t.Tag,
  squash: t.Squash,
  mixed: List<t.MixedCheque>,
  p_total: Amount,
  pendings: List<t.Pending>,
  return: fn(Amount, Option<Timestamp>) -> result,
) -> result {
  let wellsinged =
    wf.wellsigned(key, tag, _, _)
  let (squash_body, squash_signature) = squash
  expect wellsinged(squash_body, squash_signature)
  let (amount, index, exclude) = squash_body
  let with =
    with_(wellsinged, _, _, _, _, _, _, _, _)
  let upper_bound, amount, _index, _exclude, p_total, pendings <-
    foldl6(mixed, None, amount, index, exclude, p_total, pendings, with, _)
  expect p_total == 0
  expect pendings == []
  return(amount, upper_bound)
}

type ReturnWith<result> =
  fn(Option<Timestamp>, Amount, Index, t.Exclude, Amount, List<t.Pending>) ->
    result

fn with_(
  wellsinged: fn(Data, Signature) -> Bool,
  mixed_cheque: t.MixedCheque,
  upper_bound: Option<Timestamp>,
  total: Amount,
  index: Index,
  exclude: t.Exclude,
  p_total: Amount,
  pendings: List<t.Pending>,
  return: ReturnWith<result>,
) -> result {
  when mixed_cheque is {
    t.MUnlocked(body, signature, secret) -> {
      let new_index, amount, timeout <-
        wf.unlocked(wellsinged, body, signature, secret, _)
      let index, exclude <- next_index(index, exclude, new_index)
      let upper_bound = xoption.min(upper_bound, timeout)
      return(upper_bound, total + amount, index, exclude, p_total, pendings)
    }
    t.MCheque(body, signature) -> {
      expect wellsinged(body, signature)
      let (new_index, amount, timeout, lock) = body
      let index, exclude <- next_index(index, exclude, new_index)
      expect [fst, ..pendings] = pendings
      expect fst == (amount, timeout, lock)
      return(upper_bound, total, index, exclude, p_total - amount, pendings)
    }
  }
}
