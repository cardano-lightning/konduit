use aiken/crypto.{Signature, VerificationKey}
use aiken/interval.{Finite}
use aiken/primitive/int
use cardano/assets.{Assets}
use extra/xlist.{foldl5}
use konduit/assets.{amount_in, amount_out} as k_assets
use konduit/prelude.{Amount, Index, Timebound}
use konduit/steps/common.{next_index}
use konduit/types as t
use konduit/wellformed as wf

pub fn do(
  upper_bound: Timebound,
  constants: t.Constants,
  value_in: Assets,
  stage_in: t.Stage,
  value_out: Assets,
  stage_out: t.Stage,
  squash: t.Squash,
  unlockeds: List<t.Unlocked>,
) -> VerificationKey {
  trace @"sub.do": @"starting sub step"
  let (tag, add_vkey, sub_vkey, _close_period) = constants
  expect t.Opened(subbed_in, useds_in) = stage_in
  expect t.Opened(subbed_out, useds_out) = stage_out
  expect subbed_out - subbed_in == amount_in(value_in) - amount_out(value_out)
  let amount = account(
    upper_bound,
    add_vkey,
    tag,
    useds_in,
    useds_out,
    squash,
    unlockeds,
  )
  expect subbed_out <= amount
  sub_vkey
}

fn account(
  upper_bound: Timebound,
  key: VerificationKey,
  tag: t.Tag,
  useds_in: List<t.Used>,
  useds_out: List<t.Used>,
  squash: t.Squash,
  unlockeds: List<t.Unlocked>,
) -> Amount {
  trace @"receipt.0": @"squash is well-signed"
  let (squash_body, squash_signature) = squash
  // Note: Aiken 1.24 seems to not permit different types of `body`, even though `body : Data`.
  expect wf.wellsigned(key, tag, squash_body, squash_signature)
  let (owed, index, exclude) = squash_body
  let with = with_(
    upper_bound,
    wf.wellsigned(key, tag, _, _),
    _,
    _,
    _,
    _,
    _,
    _,
    _,
  )
  let owed, _useds_in, _unlockeds, _index, _exclude <- foldl5(
    useds_out,
    owed,
    useds_in,
    unlockeds,
    index,
    exclude,
    with,
    _,
  )
  owed
}

fn with_(
  upper_bound: Timebound,
  wellsinged: fn(Data, Signature) -> Bool,
  used: t.Used,
  owed: Amount,
  useds_in: List<t.Used>,
  unlockeds: List<t.Unlocked>,
  index: Index,
  exclude: t.Exclude,
  return: fn(Amount, List<t.Used>, List<t.Unlocked>, Index, t.Exclude) -> result,
) -> result {
  let (idx, _) = used
  let useds_in = useds_in |> xlist.drop_until(fn(u) { u.1st < idx })
  let handle_unlocked = fn() {
    trace @"unlocked.0": @"Expect unlocked"
    expect [unlocked, ..unlockeds] = unlockeds
    let (body, signature) = unlocked
    trace @"unlocked.1": @"Unlocked is well-formed"
    let new_index, amount, timeout <- wf.unlocked(
      wellsinged,
      body,
      signature,
      _,
    )
    trace @"unlocked.2": @"Unlocked is well-formed wrt upper bound"
    expect Finite(upper_bound) = upper_bound
    expect upper_bound <= timeout
    trace @"unlocked.3": @"Unlocked is unaccounted for"
    let index, exclude <- next_index(index, exclude, new_index)
    expect (new_index, amount) == used
    return(amount + owed, useds_in, unlockeds, index, exclude)
  }
  when useds_in is {
    [] -> {
      trace @"used.0": @"Empty useds in"
      handle_unlocked()
    }
    [used_in, ..rest] ->
      when int.compare(used_in.1st, used.1st) is {
        Less -> {
          trace @"used_in.0": @"Drop used in"
          return(owed, rest, unlockeds, index, exclude)
        }
        Equal -> {
          trace @"used_in.1": @"Persist used in"
          let (new_index, amount) = used_in
          expect used_in == used
          let index, exclude <- next_index(index, exclude, new_index)
          return(owed + amount, rest, unlockeds, index, exclude)
        }
        Greater -> {
          trace @"used_in.2": @"Handle unlocked"
          handle_unlocked()
        }
      }
  }
}
