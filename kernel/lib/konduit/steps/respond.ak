use aiken/crypto.{VerificationKey}
use aiken/interval.{Finite}
use cardano/assets.{Value}
use extra/xlist.{foldl6}
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
  cheques: List<t.Cheque>,
) -> VerificationKey {
  let t.Constants { tag, add_vkey, sub_vkey, .. } = constants
  expect t.Closed(subbed_in, useds_in, _) = stage_in
  expect t.Responded(pends_amount, pends) = stage_out
  let subbed_out = subbed_in - amount_in(value_in) + amount_out(value_out)
  let owed =
    account(
      upper_bound,
      add_vkey,
      tag,
      useds_in,
      squash,
      cheques,
      pends_amount,
      pends,
    )
  expect owed >= subbed_out
  sub_vkey
}

fn account(
  upper_bound: Timebound,
  key: VerificationKey,
  tag: t.Tag,
  useds: List<t.Used>,
  squash: t.Squash,
  cheques: List<t.Cheque>,
  p_total: Amount,
  pendings: List<t.Pending>,
) -> Amount {
  let wellsinged =
    wf.wellsigned(key, tag, _, _)
  let (squash_body, squash_signature) = squash
  expect wellsinged(squash_body, squash_signature)
  let (owed, index, exclude) = squash_body

  let before =
    fn(timeout: Int) {
      expect Finite(upper_bound) = upper_bound
      upper_bound <= timeout
    }
  let with =
    with_(before, wellsinged, _, _, _, _, _, _, _, _)
  let
    owed,
    _useds,
    _index,
    _exclude,
    p_total,
    pendings,
  <- foldl6(cheques, owed, useds, index, exclude, p_total, pendings, with)
  expect p_total == 0
  expect pendings == []
  owed
}

type ReturnWith<result> =
  fn(Amount, List<t.Used>, Index, t.Exclude, Amount, List<t.Pending>) -> result

fn with_(
  before: fn(Int) -> Bool,
  wellsinged: fn(Data, Signature) -> Bool,
  cheques: t.Cheque,
  owed: Amount,
  useds: List<t.Used>,
  index: Index,
  exclude: t.Exclude,
  p_total: Amount,
  pendings: List<t.Pending>,
  return: ReturnWith<result>,
) -> result {
  when cheques is {
    t.UnlockedCheque(body, signature, secret) -> {
      let new_index, amount, timeout <-
        wf.unlocked(wellsinged, body, signature, secret, _)
      let
        owed,
        useds,
        index,
        exclude,
      <- reduce_useds(useds, new_index, owed, index, exclude)
      let index, exclude <- next_index(index, exclude, new_index)
      expect before(timeout)
      return(owed + amount, useds, index, exclude, p_total, pendings)
    }
    t.LockedCheque(body, signature) -> {
      expect wellsinged(body, signature)
      let (new_index, amount, timeout, lock) = body
      let
        owed,
        useds,
        index,
        exclude,
      <- reduce_useds(useds, new_index, owed, index, exclude)
      let index, exclude <- next_index(index, exclude, new_index)
      expect [fst, ..pendings] = pendings
      expect fst == (amount, timeout, lock)
      return(owed, useds, index, exclude, p_total - amount, pendings)
    }
  }
}

type Fn<result> =
  fn(Amount, List<t.Used>, Index, t.Exclude) -> result

fn reduce_useds(
  self: List<t.Used>,
  ub: Index,
  total: Amount,
  lb: Index,
  exclude: t.Exclude,
  return: Fn<result>,
) -> result {
  when self is {
    [] -> return(total, [], lb, exclude)
    [x, ..tail] -> {
      let (index, amount) = x
      if index > ub {
        return(total, self, lb, exclude)
      } else {
        let lb, exclude <- next_index(lb, exclude, index)
        reduce_useds(tail, ub, total + amount, lb, exclude, return)
      }
    }
  }
}

test test_reduce_useds_0() {
  let useds = []
  let ub = 1
  let total = 0
  let lb = 0
  let exclude = []
  let
    total,
    _useds,
    _index,
    _exclude,
  <- reduce_useds(useds, ub, total, lb, exclude)
  total == 0
}

test test_reduce_useds_1() {
  let useds = [(1, 100)]
  let ub = 1
  let total = 0
  let lb = 0
  let exclude = []
  let
    total,
    useds,
    index,
    _exclude,
  <- reduce_useds(useds, ub, total, lb, exclude)
  expect total == 100
  expect useds == []
  expect index == 1
  expect exclude == []
}

test test_reduce_useds_2() {
  let useds = [(1, 100), (2, 100)]
  let ub = 1
  let total = 0
  let lb = 0
  let exclude = []
  let
    total,
    useds,
    index,
    _exclude,
  <- reduce_useds(useds, ub, total, lb, exclude)
  expect total == 100
  expect useds == [(2, 100)]
  expect index == 1
}

test test_reduce_useds_3() {
  let useds = [(1, 100), (2, 100), (4, 100)]
  let ub = 3
  let total = 0
  let lb = 0
  let exclude = []
  let
    total,
    useds,
    index,
    _exclude,
  <- reduce_useds(useds, ub, total, lb, exclude)
  expect total == 200
  expect useds == [(4, 100)]
  expect index == 2
}

test test_reduce_useds_exclude() {
  let useds = [(1, 100), (2, 100), (4, 100)]
  let ub = 5
  let total = 0
  let lb = 3
  let exclude = [1, 2]
  let
    total,
    useds,
    index,
    _exclude,
  <- reduce_useds(useds, ub, total, lb, exclude)
  expect total == 300
  expect useds == []
  expect index == 4
}
