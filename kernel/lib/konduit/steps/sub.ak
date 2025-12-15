use aiken/collection/list
use aiken/crypto.{Signature, VerificationKey}
use aiken/interval.{Finite}
use aiken/primitive/int
use cardano/assets.{Value}
use extra/xlist.{foldl4}
use konduit/assets.{amount_in, amount_out} as k_assets
use konduit/prelude.{Amount, Index, Timebound}
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
  unlockeds: List<t.Unlocked>,
) -> VerificationKey {
  trace @"sub.do": @"starting sub step"
  let t.Constants { tag, add_vkey, sub_vkey, .. } = constants
  expect t.Opened(subbed_in, useds_in) = stage_in
  expect t.Opened(subbed_out, useds_out) = stage_out
  expect subbed_out - subbed_in == amount_in(value_in) - amount_out(value_out)
  let amount =
    account(upper_bound, useds_in, useds_out, add_vkey, tag, squash, unlockeds)
  expect subbed_out <= amount
  sub_vkey
}

fn account(
  upper_bound: Timebound,
  useds_in: List<t.Used>,
  useds_out: List<t.Used>,
  key: VerificationKey,
  tag: t.Tag,
  squash: t.Squash,
  unlockeds: List<t.Unlocked>,
) -> Amount {
  trace @"receipt.0": @"squash is well-signed"
  let wellsinged =
    wf.wellsigned(key, tag, _, _)
  let (squash_body, squash_signature) = squash
  expect wellsinged(squash_body, squash_signature)
  let (amount, index, exclude) = squash_body
  // useds = useds_out - (useds_in . keep_if(not_squashed)) 
  let useds = reduce_useds(index, exclude, useds_in, useds_out)
  let with =
    with_(upper_bound, wellsinged, _, _, _, _, _, _)
  let amount, _index, _exclude, useds <-
    foldl4(unlockeds, amount, index, exclude, useds, with, _)
  expect useds == []
  amount
}

fn with_(
  upper_bound: Timebound,
  wellsinged: fn(Data, Signature) -> Bool,
  unlocked: t.Unlocked,
  total: Amount,
  index: Index,
  exclude: t.Exclude,
  useds: List<t.Used>,
  return: fn(Amount, Index, t.Exclude, List<t.Used>) -> result,
) -> result {
  let (body, signature, secret) = unlocked
  trace @"receipt.1": @"each unlocked is well-formed"
  let new_index, amount, timeout <-
    wf.unlocked(wellsinged, body, signature, secret, _)
  trace @"receipt.2": @"each unlocked is unaccounted for in the squash"
  let index, exclude <- next_index(index, exclude, new_index)
  expect Finite(upper_bound) = upper_bound
  expect upper_bound < timeout
  expect [(index_, amount_), ..useds] = useds
  expect index == index_
  expect amount == amount_
  return(total + amount, index, exclude, useds)
}

fn reduce_useds(
  squash_index: Index,
  exclude: t.Exclude,
  useds_in: List<t.Used>,
  useds_out: List<t.Used>,
) {
  when useds_in is {
    [] -> useds_out
    [used, ..useds_in] ->
      if is_accounted_for(squash_index, exclude, used.1st) {
        reduce_useds(squash_index, exclude, useds_in, useds_out)
      } else {
        let useds_out = useds_out |> list.drop_while(fn(x) { x != used })
        reduce_useds(squash_index, exclude, useds_in, useds_out)
      }
  }
}

fn is_accounted_for(squash_index: Index, exclude: t.Exclude, index: Index) {
  when int.compare(squash_index, index) is {
    Greater -> False
    Equal -> True
    Less -> exclude |> list.has(index)
  }
}
