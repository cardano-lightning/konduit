use aiken/crypto.{Signature, VerificationKey}
use aiken/interval.{Finite}
use aiken/primitive/int
use cardano/assets.{Value}
use extra/xlist.{foldl5}
use konduit/assets.{amount_in, amount_out, preserve_lovelace} as k_assets
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
  let (tag, add_vkey, sub_vkey, _close_period, asset) = constants
  expect t.Opened(subbed_in, useds_in) = stage_in
  expect t.Opened(subbed_out, useds_out) = stage_out
  expect preserve_lovelace(value_in, value_out, asset)
  expect
    subbed_out - subbed_in == amount_in(value_in, asset) - amount_out(
      value_out,
      asset,
    )
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
  let wellsinged = wf.wellsigned(key, tag, _, _)
  let (squash_body, squash_signature) = squash
  expect wellsinged(squash_body, squash_signature)
  let (owed, index, exclude) = squash_body
  let with = with_(upper_bound, wellsinged, _, _, _, _, _, _, _)
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

const b32 = #"0000000000000000000000000000000000000000000000000000000000000000"

// Ed25519 fixtures use seed bytes 00..1f and CBOR (tag, body), with indefinite
// nonempty arrays and definite empty arrays. Cheque bodies sign SHA2-256(b32).
const key = #"03a107bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b8"

const sig =
  #"9546a3c9e0f1786d73aedbb634ed603dae9140b17221d9b422b1403049ab382d3a7dff0696562bd11bdff7d0165d9f2450c8fb8885a6b13a3ee7ec09b0ae260d"

const sig_squash_1 =
  #"08c985183453e0a7f3208b11d27922e291726a83044529f6641bebdd60a1f960ad94bdf1b38312402ad214255982ad3eeaf83ef2de7c3915fab598be5f14d004"

const sig_squash_2 =
  #"8864713eb2727be0799b68e3e64b6d8c5e2c2e4f90af6bf16f926241bf117e355d3eb6da9ac3702b1267788f1ab5e0e0a3cdfff2687b9e6465222434fc08470a"

const sig_squash_exclude =
  #"950344709cecf0ca187338ccc3c47df751f3d5d577594f4a937311db7ce6244f7103a304f6036f0d33f90de852857931a40e2ae516d01bdd03d5b48139c6a303"

const sig_squash_big =
  #"af9bf18331efa7e398cf430e75c57c58f511a44ae3125b902202c54709c644fdec4b826b08dbd01c407f3be2862e46f5fe1e89e79984e361dc7c1c7f4cdd8609"

const sig_cheque_2 =
  #"e60c1aa1f02e7c13844200399d27e187be8bdd619e4f4ea84ee8b62dff6aeaa9786b39af770a2b896f400e43de59a0db6f4bba57f4fc5eaee9788dd91e80df02"

const sig_cheque_4 =
  #"ddee8c1807c071feff04797e301e7e46b7ac2d14ffb3538802e0fa162b5a6b62c8191c8ef9f0d47c3f100c093ff7f690edefec02de60bb474c5ec0ca747d1a09"

const sig_cheque_6 =
  #"ed61cbe06d0eab0d7a32749383f75205b63e7d1994cad99d0b347f884bc44224f0ed9775c927c0ea4dea2496f1b84a45a75031f5e4e3ad11869a9310222ed40c"

const sig_cheque_80 =
  #"24dc6b6d2472828f1854f5a455e8b81e67c7496af19c67f2eef3bfdd2ba965735c638398e0d79e9e7da1423d00a943a6155767e58bb47e2cf8b250360e68020d"

const sig_cheque_84 =
  #"bd2dfbec4b5d36535f9d8cd1e7521e10a29211a3faffda807196bec9d2a82124e74b9f501afa918b33b9a8c891c05d4827b7babf030268a873f0fdd7be9e5501"

const sig_cheque_88 =
  #"ecf93572185accdc0c13c088a3c4f4080834a33144d02e81e5536292c7fd8ad99b43c55afa81ea9ee458be72a0d328dbfdd16c29081b3850b5c1ea5ecd1c9509"

const sig_cheque_92 =
  #"bd96d70edcea01acbcb17999ad42594310c97cd73ddef8103f1358e5359592a424562da1977d8695c40a8d75b2a8b091af5f308600879138e7fa5e1d648b1b0d"

const sig_cheque_96 =
  #"18046059783ddf0fce70ca66b7db5c84b8fe2056742aa09959d1ddf29451d0ad0e7ac0a1c42c36ec930a8cd0cfa92028669478b296182d8d93eafea631ec3207"

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
  let unlockeds = [
    ((2, 100, later, b32), sig_cheque_2), ((4, 12, later, b32), sig_cheque_4),
  ]
  let tot = account_(useds_in, useds_out, null_squash, unlockeds)
  tot == 224
}

test test_account_scenario_1() {
  let useds_in = [(1, 100), (3, 12)]
  let useds_out = [(2, 100), (3, 12), (4, 12), (6, 111)]
  let unlockeds = [
    ((2, 100, later, b32), sig_cheque_2), ((4, 12, later, b32), sig_cheque_4),
    ((6, 111, later, b32), sig_cheque_6),
  ]
  let tot = account_(useds_in, useds_out, null_squash, unlockeds)
  tot == 235
}

test test_ooo_useds_outs() fail {
  let useds_in = [(1, 100), (3, 12)]
  let useds_out = [(3, 12), (2, 100), (4, 12), (6, 111)]
  let unlockeds = [
    ((2, 100, later, b32), sig_cheque_2), ((4, 12, later, b32), sig_cheque_4),
    ((6, 111, later, b32), sig_cheque_6),
  ]
  let tot = account_(useds_in, useds_out, null_squash, unlockeds)
  tot == 235
}

test test_ooo_useds_outs_1() fail {
  let useds_in = [(1, 100), (3, 12)]
  let useds_out = [(2, 100), (3, 12), (6, 111), (4, 12)]
  let unlockeds = [
    ((2, 100, later, b32), sig_cheque_2), ((4, 12, later, b32), sig_cheque_4),
    ((6, 111, later, b32), sig_cheque_6),
  ]
  let tot = account_(useds_in, useds_out, null_squash, unlockeds)
  tot == 235
}

test test_useds_out_too_long() fail {
  let useds_in = [(1, 100), (3, 12)]
  let useds_out = [(2, 100), (3, 12), (4, 12), (6, 111)]
  let unlockeds = [
    ((2, 100, later, b32), sig_cheque_2), ((4, 12, later, b32), sig_cheque_4),
  ]
  let tot = account_(useds_in, useds_out, null_squash, unlockeds)
  tot == 235
}

test test_with_squash() {
  let squash = ((1000, 1, []), sig_squash_1)
  let useds_in = [(1, 100), (3, 12)]
  let useds_out = [(2, 100), (3, 12), (4, 12)]
  let unlockeds = [
    ((2, 100, later, b32), sig_cheque_2), ((4, 12, later, b32), sig_cheque_4),
  ]
  let tot = account_(useds_in, useds_out, squash, unlockeds)
  tot == 1124
}

test test_used_already_squashed() fail {
  let squash = ((1000, 2, []), sig_squash_2)
  let useds_in = [(2, 100), (3, 12)]
  let useds_out = [(2, 100), (3, 12), (4, 12)]
  let unlockeds = [((4, 12, later, b32), sig_cheque_4)]
  let tot = account_(useds_in, useds_out, squash, unlockeds)
  tot == 1124
}

test test_used_not_squashed() {
  let squash = ((1000, 3, [2]), sig_squash_exclude)
  let useds_in = [(2, 100)]
  let useds_out = [(2, 100), (4, 12)]
  let unlockeds = [((4, 12, later, b32), sig_cheque_4)]
  let tot = account_(useds_in, useds_out, squash, unlockeds)
  tot == 1112
}

test test_unlocked_already_used() {
  let squash = ((1000, 3, [2]), sig_squash_exclude)
  let useds_in = [(2, 100)]
  let useds_out = [(2, 100)]
  let unlockeds = [((2, 100, later, b32), sig_cheque_2)]
  let tot = account_(useds_in, useds_out, squash, unlockeds)
  tot == 1100
}

test test_big_scenario() {
  let squash = (
    (10000000, 100, [80, 82, 84, 86, 88, 90, 92, 94, 96]),
    sig_squash_big,
  )
  let useds_in = [(81, 1), (82, 1), (83, 1), (85, 1), (86, 1), (87, 1)]
  let useds_out = [
    (80, 100), (82, 1), (84, 100), (86, 1), (88, 100), (92, 100), (96, 100),
  ]
  let unlockeds = [
    ((80, 100, later, b32), sig_cheque_80),
    ((84, 100, later, b32), sig_cheque_84),
    ((88, 100, later, b32), sig_cheque_88),
    ((92, 100, later, b32), sig_cheque_92),
    ((96, 100, later, b32), sig_cheque_96),
  ]
  let tot = account_(useds_in, useds_out, squash, unlockeds)
  tot == 10000502
}
