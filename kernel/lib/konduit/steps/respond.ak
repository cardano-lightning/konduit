use aiken/crypto.{VerificationKey}
use aiken/interval.{Finite}
use cardano/assets.{Value}
use extra/xlist.{foldl5}
use konduit/assets.{amount_in, amount_out} as k_assets
use konduit/prelude.{Amount, Index, Signature, Timebound}
use konduit/steps/common.{next_index}
use konduit/types as t
use konduit/wellformed as wf

pub fn do(
  upper_bound: Timebound,
  constants: t.Constants,
  value_in: Value,
  stage_in: t.Stage,
  value_out: Value,
  stage_out: t.Stage,
  squash: t.Squash,
  mixeds: List<t.MixedCheque>,
) -> VerificationKey {
  let t.Constants { tag, add_vkey, sub_vkey, .. } = constants
  expect t.Closed(subbed_in, useds_in, _) = stage_in
  expect t.Responded(pends_amount, pends) = stage_out
  let subbed_out = subbed_in - amount_in(value_in) + amount_out(value_out)
  let amount =
    account(upper_bound, add_vkey, tag, squash, mixeds, pends_amount, pends)
  expect amount >= subbed_out
  sub_vkey
}

fn account(
  upper_bound: Timebound,
  key: VerificationKey,
  tag: t.Tag,
  squash: t.Squash,
  mixed: List<t.MixedCheque>,
  p_total: Amount,
  pendings: List<t.Pending>,
) -> Amount {
  let wellsinged =
    wf.wellsigned(key, tag, _, _)
  let (squash_body, squash_signature) = squash
  expect wellsinged(squash_body, squash_signature)
  let (amount, index, exclude) = squash_body
  let with =
    with_(upper_bound, wellsinged, _, _, _, _, _, _, _)
  let amount, _index, _exclude, p_total, pendings <-
    foldl5(mixed, amount, index, exclude, p_total, pendings, with, _)
  expect p_total == 0
  expect pendings == []
  amount
}

type ReturnWith<result> =
  fn(Amount, Index, t.Exclude, Amount, List<t.Pending>) -> result

fn with_(
  upper_bound: Timebound,
  wellsinged: fn(Data, Signature) -> Bool,
  mixed_cheque: t.MixedCheque,
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
      expect Finite(upper_bound) = upper_bound
      expect upper_bound <= upper_bound
      return(total + amount, index, exclude, p_total, pendings)
    }
    t.MCheque(body, signature) -> {
      expect wellsinged(body, signature)
      let (new_index, amount, timeout, lock) = body
      let index, exclude <- next_index(index, exclude, new_index)
      expect [fst, ..pendings] = pendings
      expect fst == (amount, timeout, lock)
      return(total, index, exclude, p_total - amount, pendings)
    }
  }
}
