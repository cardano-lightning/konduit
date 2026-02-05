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
    account(upper_bound, add_vkey, tag, useds_in, useds_out, squash, unlockeds)
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
  let wellsinged =
    wf.wellsigned(key, tag, _, _)
  let (squash_body, squash_signature) = squash
  expect wellsinged(squash_body, squash_signature)
  let (owed, index, exclude) = squash_body
  let with =
    with_(upper_bound, wellsinged, _, _, _, _, _, _, _)
  let owed, _useds_in, _unlockeds, _index, _exclude <-
    foldl5(useds_out, owed, useds_in, unlockeds, index, exclude, with, _)
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

const b64 =
  #"00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"

const b32 = #"0000000000000000000000000000000000000000000000000000000000000000"

const key = b32

const sig = b64

const now = 1_765_889_000_000

const soon = now + 60_000

const ub = Finite(soon)

const later = soon + 60_000

const tag = "test"

const null_squash = ((0, 0, []), sig)

const account_ = account(ub, key, tag, _, _, _, _)

test test_account_null() {
  let tot = account_([], [], null_squash, [])
  tot == 0
}

test test_account_scenario_0() {
  let useds_in = [(1, 100), (3, 12)]
  let useds_out = [(1, 100), (2, 100), (3, 12), (4, 12)]
  let unlockeds =
    [((2, 100, later, b32), b64, b32), ((4, 12, later, b32), b64, b32)]
  let tot = account_(useds_in, useds_out, null_squash, unlockeds)
  tot == 224
}

test test_account_scenario_1() {
  let useds_in = [(1, 100), (3, 12)]
  let useds_out = [(2, 100), (3, 12), (4, 12), (6, 111)]
  let unlockeds =
    [
      ((2, 100, later, b32), b64, b32), ((4, 12, later, b32), b64, b32),
      ((6, 111, later, b32), b64, b32),
    ]
  let tot = account_(useds_in, useds_out, null_squash, unlockeds)
  tot == 235
}

test test_ooo_useds_outs() fail {
  let useds_in = [(1, 100), (3, 12)]
  let useds_out = [(3, 12), (2, 100), (4, 12), (6, 111)]
  let unlockeds =
    [
      ((2, 100, later, b32), b64, b32), ((4, 12, later, b32), b64, b32),
      ((6, 111, later, b32), b64, b32),
    ]
  let tot = account_(useds_in, useds_out, null_squash, unlockeds)
  tot == 235
}

test test_ooo_useds_outs_1() fail {
  let useds_in = [(1, 100), (3, 12)]
  let useds_out = [(2, 100), (3, 12), (6, 111), (4, 12)]
  let unlockeds =
    [
      ((2, 100, later, b32), b64, b32), ((4, 12, later, b32), b64, b32),
      ((6, 111, later, b32), b64, b32),
    ]
  let tot = account_(useds_in, useds_out, null_squash, unlockeds)
  tot == 235
}

test test_useds_out_too_long() fail {
  let useds_in = [(1, 100), (3, 12)]
  let useds_out = [(2, 100), (3, 12), (4, 12), (6, 111)]
  let unlockeds =
    [((2, 100, later, b32), b64, b32), ((4, 12, later, b32), b64, b32)]
  let tot = account_(useds_in, useds_out, null_squash, unlockeds)
  tot == 235
}

test test_with_squash() {
  let squash = ((1000, 1, []), sig)
  let useds_in = [(1, 100), (3, 12)]
  let useds_out = [(2, 100), (3, 12), (4, 12)]
  let unlockeds =
    [((2, 100, later, b32), b64, b32), ((4, 12, later, b32), b64, b32)]
  let tot = account_(useds_in, useds_out, squash, unlockeds)
  tot == 1124
}

test test_used_already_squashed() fail {
  let squash = ((1000, 2, []), sig)
  let useds_in = [(2, 100), (3, 12)]
  let useds_out = [(2, 100), (3, 12), (4, 12)]
  let unlockeds = [((4, 12, later, b32), b64, b32)]
  let tot = account_(useds_in, useds_out, squash, unlockeds)
  tot == 1124
}

test test_used_not_squashed() {
  let squash = ((1000, 3, [2]), sig)
  let useds_in = [(2, 100)]
  let useds_out = [(2, 100), (4, 12)]
  let unlockeds = [((4, 12, later, b32), b64, b32)]
  let tot = account_(useds_in, useds_out, squash, unlockeds)
  tot == 1112
}

test test_unlocked_already_used() fail {
  let squash = ((1000, 3, [2]), sig)
  let useds_in = [(2, 100)]
  let useds_out = [(2, 100)]
  let unlockeds = [((2, 100, later, b32), b64, b32)]
  let tot = account_(useds_in, useds_out, squash, unlockeds)
  tot == 1100
}

test test_big_scenario() {
  let squash = ((10000000, 100, [80, 82, 84, 86, 88, 90, 92, 94, 96]), sig)
  let useds_in = [(81, 1), (82, 1), (83, 1), (85, 1), (86, 1), (87, 1)]
  let useds_out =
    [(80, 100), (82, 1), (84, 100), (86, 1), (88, 100), (92, 100), (96, 100)]
  let unlockeds =
    [
      ((80, 100, later, b32), b64, b32), ((84, 100, later, b32), b64, b32),
      ((88, 100, later, b32), b64, b32), ((92, 100, later, b32), b64, b32),
      ((96, 100, later, b32), b64, b32),
    ]
  let tot = account_(useds_in, useds_out, squash, unlockeds)
  tot == 10000502
}
