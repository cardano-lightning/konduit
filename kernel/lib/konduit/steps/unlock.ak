use aiken/builtin.{head_list, sha2_256, tail_list}
use aiken/primitive/bytearray
use cardano/assets.{Value}
use extra/xlist.{foldl4}
use extra/xoption
use konduit/assets.{amount_in, amount_out} as k_assets
use konduit/prelude.{Amount, Timestamp}
use konduit/steps/common.{Return}
use konduit/types as t

pub fn do(
  constants: t.Constants,
  value_in: Value,
  stage_in: t.Stage,
  value_out: Value,
  stage_out: t.Stage,
  unpends: List<t.Unpend>,
  return: Return<result>,
) -> result {
  let t.Constants { sub_vkey, .. } = constants
  expect t.Responded(pends_amount_in, pends_in) = stage_in
  expect t.Responded(pends_amount_out, pends_out) = stage_out
  let subbed = amount_in(value_in) - amount_out(value_out)
  let amount, upper_bound <- account(unpends, pends_in, pends_out)
  expect pends_amount_out == pends_amount_in - amount
  expect amount >= subbed
  return(sub_vkey, None, upper_bound)
}

fn account(
  unpends: List<t.Unpend>,
  pends_in: List<t.Pending>,
  pends_out: List<t.Pending>,
  return: fn(Amount, Option<Timestamp>) -> result,
) -> result {
  let timeout, amount, pends_in, pends_out <-
    foldl4(unpends, None, 0, pends_in, pends_out, with, _)
  expect pends_in == pends_out
  return(amount, timeout)
}

type ReturnWith<result> =
  fn(Option<Timestamp>, Amount, List<t.Pending>, List<t.Pending>) -> result

fn with(
  unpend: t.Unpend,
  upper_bound: Option<Timestamp>,
  total: Amount,
  pends_in: List<t.Pending>,
  pends_out: List<t.Pending>,
  return: ReturnWith<result>,
) -> result {
  when unpend is {
    #"" -> {
      // Continue
      expect head_list(pends_in) == head_list(pends_out)
      return(upper_bound, total, tail_list(pends_in), tail_list(pends_out))
    }
    _ -> {
      // Unlock
      let (amount, timeout, hash) = head_list(pends_in)
      expect sha2_256(unpend) == hash
      expect bytearray.length(unpend) == 32
      let upper_bound = xoption.min(upper_bound, timeout)
      return(upper_bound, total + amount, tail_list(pends_in), pends_out)
    }
  }
}
