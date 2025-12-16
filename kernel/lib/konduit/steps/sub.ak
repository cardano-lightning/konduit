use aiken/collection/list
use aiken/crypto.{Signature, VerificationKey}
use aiken/interval.{Finite}
use aiken/primitive/int
use cardano/assets.{Value}
use extra/xlist.{foldl5}
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
  let with =
    with_(upper_bound, wellsinged, _, _, _, _, _, _, _)
  let amount, _useds_in, unlockeds, _index, _exclude <-
    foldl5(useds_out, amount, useds_in, unlockeds, index, exclude, with, _)
  expect unlockeds == []
  amount
}

fn with_(
  upper_bound: Timebound,
  wellsinged: fn(Data, Signature) -> Bool,
  used: t.Used,
  total: Amount,
  useds_in: List<t.Used>,
  unlockeds: List<t.Unlocked>,
  index: Index,
  exclude: t.Exclude,
  return: fn(Amount, List<t.Used>, List<t.Unlocked>, Index, t.Exclude) -> result,
) -> result {
  let (used_index, _) = used
  trace (useds_in, used_index)
  let useds_in =
    useds_in
      |> xlist.drop_until(
          fn(used) {
            trace @"DROP"
            used.1st < used_index
          },
        )
  let handle_unlocked =
    fn() {
      trace @"unlocked.0": @"Expect unlocked"
      expect [unlocked, ..unlockeds] = unlockeds
      let (body, signature, secret) = unlocked
      trace @"unlocked.1": @"Unlocked is well-formed"
      let new_index, amount, timeout <-
        wf.unlocked(wellsinged, body, signature, secret, _)
      trace @"unlocked.2": @"Unlocked is well-formed wrt upper bound"
      expect Finite(upper_bound) = upper_bound
      expect upper_bound < timeout
      trace @"unlocked.3": @"Unlocked is unaccounted for"
      let index, exclude <- next_index(index, exclude, new_index)
      expect (new_index, amount) == used
      return(amount + total, useds_in, unlockeds, index, exclude)
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
          return(total, rest, unlockeds, index, exclude)
        }
        Equal -> {
          trace @"used_in.1": @"Persist used in"
          let (new_index, amount) = used_in
          expect used_in == used
          let index, exclude <- next_index(index, exclude, new_index)
          return(total + amount, rest, unlockeds, index, exclude)
        }
        Greater -> {
          trace @"used_in.2": @"Handle unlocked"
          handle_unlocked()
        }
      }
  }
}

test test_account_null() {
  let upper_bound: Timebound = Finite(1_000_000)
  let useds_in: List<t.Used> = []
  let useds_out: List<t.Used> = []
  let key: VerificationKey =
    #"0000000000000000000000000000000000000000000000000000000000000000"
  let tag: t.Tag = "my_test"
  let amount = 0
  let index = 0
  let exclude = []
  let signature =
    #"00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"
  let squash: t.Squash = ((amount, index, exclude), signature)
  let unlockeds: List<t.Unlocked> = []
  let tot =
    account(upper_bound, useds_in, useds_out, key, tag, squash, unlockeds)
  tot == 0
}

const b64 =
  #"00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"

const b32 = #"0000000000000000000000000000000000000000000000000000000000000000"

const now = 1_765_889_000_000

test test_account_persist_1() {
  let upper_bound: Timebound = Finite(now + 1_000)
  let useds_in: List<t.Used> = [(1, 100), (3, 12)]
  let useds_out: List<t.Used> = [(1, 100), (2, 100), (3, 12), (4, 12)]
  let key: VerificationKey = b32
  let tag: t.Tag = "my_test"
  let amount = 0
  let index = 0
  let exclude = []
  let squash: t.Squash = ((amount, index, exclude), b64)
  let unlockeds: List<t.Unlocked> = [((2, 100, now + 2_000, ""), "", b32)]
  let tot =
    account(upper_bound, useds_in, useds_out, key, tag, squash, unlockeds)
  tot == 200
}

test test_account_no_persist_1() {
  let upper_bound: Timebound = Finite(now + 1_000)
  let useds_in: List<t.Used> = [(1, 100)]
  let useds_out: List<t.Used> = [(2, 100)]
  let key: VerificationKey = b32
  let tag: t.Tag = "my_test"
  let amount = 0
  let index = 0
  let exclude = []
  let squash: t.Squash = ((amount, index, exclude), b64)
  let unlockeds: List<t.Unlocked> = [((2, 100, now + 2_000, ""), "", b32)]
  let tot =
    account(upper_bound, useds_in, useds_out, key, tag, squash, unlockeds)
  tot == 100
}
