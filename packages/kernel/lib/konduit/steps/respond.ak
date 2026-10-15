use aiken/collection/list
use aiken/crypto.{VerificationKey}
use aiken/interval.{Finite}
use cardano/assets.{Value}
use extra/xlist.{foldl6}
use konduit/assets.{amount_in, amount_out, preserve_lovelace} as k_assets
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
  let (tag, add_vkey, sub_vkey, _close_period, asset) = constants
  expect t.Closed(subbed_in, useds_in, elapse_at) = stage_in
  expect Finite(upper) = upper_bound
  expect upper <= elapse_at
  expect t.Responded(pends_amount, pends) = stage_out
  expect preserve_lovelace(value_in, value_out, asset)
  let subbed_out = subbed_in + amount_in(value_in, asset) - amount_out(
    value_out,
    asset,
  )
  let owed = account(
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
  let wellsinged = wf.wellsigned(key, tag, _, _)
  let (squash_body, squash_signature) = squash
  expect wellsinged(squash_body, squash_signature)
  let (owed, index, exclude) = squash_body

  let before = fn(timeout: Int) {
    expect Finite(upper_bound) = upper_bound
    upper_bound <= timeout
  }
  let with = with_(before, wellsinged, _, _, _, _, _, _, _, _)
  let owed, useds, index, exclude, p_total, pendings <- foldl6(
    cheques,
    owed,
    useds,
    index,
    exclude,
    p_total,
    pendings,
    with,
  )
  expect p_total == 0
  expect pendings == []
  let owed, _useds, _index, _exclude <- reduce_useds(
    useds,
    None,
    owed,
    index,
    exclude,
  )
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
    t.UnlockedCheque(body, signature) -> {
      let new_index, amount, timeout <- wf.unlocked(
        wellsinged,
        body,
        signature,
        _,
      )
      let owed, useds, index, exclude <- reduce_useds(
        useds,
        Some(new_index),
        owed,
        index,
        exclude,
      )
      let index, exclude <- next_index(index, exclude, new_index)
      expect before(timeout)
      return(owed + amount, useds, index, exclude, p_total, pendings)
    }
    t.LockedCheque(body, signature) -> {
      expect wellsinged(body, signature)
      let (new_index, amount, timeout, lock) = body
      let owed, useds, index, exclude <- reduce_useds(
        useds,
        Some(new_index),
        owed,
        index,
        exclude,
      )
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
  ub: Option<Index>,
  total: Amount,
  lb: Index,
  exclude: t.Exclude,
  return: Fn<result>,
) -> result {
  when self is {
    [] -> return(total, [], lb, exclude)
    [x, ..tail] -> {
      let (index, amount) = x
      let stop = when ub is {
        Some(ub) -> index > ub
        None -> False
      }
      if stop {
        return(total, self, lb, exclude)
      } else {
        // Used entries are ordered. Skip credit already included in the squash.
        let exclude = xlist.drop_until_int(exclude, index)
        if index > lb || list.head(exclude) == Some(index) {
          let lb, exclude <- next_index(lb, exclude, index)
          reduce_useds(tail, ub, total + amount, lb, exclude, return)
        } else {
          reduce_useds(tail, ub, total, lb, exclude, return)
        }
      }
    }
  }
}
