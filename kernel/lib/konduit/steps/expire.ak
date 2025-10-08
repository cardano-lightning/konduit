use aiken/builtin.{head_list, tail_list}
use cardano/assets.{Value}
use env
use extra/xlist.{foldl4}
use extra/xoption
use konduit/assets.{amount_out} as k_assets
use konduit/prelude.{Amount, Timestamp}
use konduit/steps/common.{Return}
use konduit/types as t

pub fn do(
  constants: t.Constants,
  stage_in: t.Stage,
  value_out: Value,
  stage_out: t.Stage,
  unpends: List<t.Unpend>,
  return: Return<result>,
) -> result {
  let t.Constants { add_vkey, .. } = constants
  expect t.Responded(pends_amount_in, pends_in) = stage_in
  expect t.Responded(pends_amount_out, pends_out) = stage_out
  let amount, lower_bound <-
    account(unpends, pends_in, pends_out, _)
  expect pends_amount_out == pends_amount_in - amount
  expect amount_out(value_out) >= pends_amount_out + env.min_ada_buffer
  return(add_vkey, lower_bound, None)
}

pub fn account(
  unpends: List<t.Unpend>,
  pends_in: List<t.Pending>,
  pends_out: List<t.Pending>,
  return: fn(Amount, Option<Timestamp>) -> result,
) {
  let lower_bound, amount, pends_in, pends_out <-
    foldl4(unpends, None, 0, pends_in, pends_out, with, _)
  expect pends_in == pends_out
  return(amount, lower_bound)
}

pub fn with(
  unpend: t.Unpend,
  lower_bound: Option<Timestamp>,
  total: Amount,
  pends_in: List<t.Pending>,
  pends_out: List<t.Pending>,
  return: fn(Option<Timestamp>, Amount, List<t.Pending>, List<t.Pending>) ->
    result,
) -> result {
  when unpend is {
    #"" -> {
      // Continue
      expect head_list(pends_in) == head_list(pends_out)
      return(lower_bound, total, tail_list(pends_in), tail_list(pends_out))
    }
    _ -> {
      // Expire
      let (amount, timeout, _) = head_list(pends_in)
      let lower_bound = xoption.max(lower_bound, timeout)
      return(lower_bound, total + amount, tail_list(pends_in), pends_out)
    }
  }
}
