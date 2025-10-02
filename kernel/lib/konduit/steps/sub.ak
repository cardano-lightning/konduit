use aiken/crypto.{Signature, VerificationKey}
use cardano/assets.{Value}
use extra/xlist.{foldl4}
use extra/xoption
use konduit/assets.{amount_in, amount_out} as k_assets
use konduit/prelude.{Amount, Index, Timestamp}
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
  unlockeds: List<t.Unlocked>,
  return: Return<result>,
) -> result {
  let t.Constants { tag, add_vkey, sub_vkey, .. } = constants
  expect t.Opened(subbed_in) = stage_in
  expect t.Opened(subbed_out) = stage_out
  expect subbed_out - subbed_in == amount_in(value_in) - amount_out(value_out)
  let amount, upper_bound <-
    account(tag, add_vkey, squash, unlockeds, _)
  expect subbed_out <= amount
  return(sub_vkey, None, upper_bound)
}

fn account(
  key: VerificationKey,
  tag: t.Tag,
  squash: t.Squash,
  unlockeds: List<t.Unlocked>,
  return: fn(Amount, Option<Timestamp>) -> result,
) -> result {
  trace @"receipt.0": @"squash is well-signed"
  let wellsinged =
    wf.wellsigned(key, tag, _, _)
  let (squash_body, squash_signature) = squash
  expect wellsinged(squash_body, squash_signature)
  let (amount, index, exclude) = squash_body
  let with =
    with_(wellsinged, _, _, _, _, _, _)
  let upper_bound, amount, _index, _exclude <-
    foldl4(unlockeds, None, amount, index, exclude, with, _)
  return(amount, upper_bound)
}

fn with_(
  wellsinged: fn(Data, Signature) -> Bool,
  unlocked: t.Unlocked,
  upper_bound: Option<Timestamp>,
  total: Amount,
  index: Index,
  exclude: t.Exclude,
  return: fn(Option<Timestamp>, Amount, Index, t.Exclude) -> result,
) -> result {
  let (body, signature, secret) = unlocked
  trace @"receipt.1": @"each unlocked is well-formed"
  let new_index, amount, timeout <-
    wf.unlocked(wellsinged, body, signature, secret, _)
  trace @"receipt.2": @"each unlocked is unaccounted for in the squash"
  let index, exclude <- next_index(index, exclude, new_index)
  return(xoption.min(upper_bound, timeout), total + amount, index, exclude)
}
