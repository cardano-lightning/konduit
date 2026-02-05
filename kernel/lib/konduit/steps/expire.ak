use aiken/builtin.{head_list, tail_list}
use aiken/crypto.{VerificationKey}
use aiken/interval.{Finite}
use cardano/assets.{Value}
use env
use extra/xlist.{foldl3}
use konduit/assets.{amount_out} as k_assets
use konduit/prelude.{Amount, Timebound}
use konduit/types as t

pub fn do(
  lower_bound: Timebound,
  constants: t.Constants,
  stage_in: t.Stage,
  value_out: Value,
  stage_out: t.Stage,
  unpends: List<t.Unpend>,
) -> VerificationKey {
  let t.Constants { add_vkey, .. } = constants
  expect t.Responded(pends_amount_in, pends_in) = stage_in
  expect t.Responded(pends_amount_out, pends_out) = stage_out
  let amount = account(lower_bound, unpends, pends_in, pends_out)
  expect pends_amount_out == pends_amount_in - amount
  expect amount_out(value_out) >= pends_amount_out + env.min_ada_buffer
  add_vkey
}

pub fn account(
  lower_bound: Timebound,
  unpends: List<t.Unpend>,
  pends_in: List<t.Pending>,
  pends_out: List<t.Pending>,
) {
  let with =
    with_(lower_bound, _, _, _, _, _)
  let amount, pends_in, pends_out <-
    foldl3(unpends, 0, pends_in, pends_out, with, _)
  expect pends_in == pends_out
  amount
}

pub fn with_(
  lower_bound: Timebound,
  unpend: t.Unpend,
  total: Amount,
  pends_in: List<t.Pending>,
  pends_out: List<t.Pending>,
  return: fn(Amount, List<t.Pending>, List<t.Pending>) -> result,
) -> result {
  when unpend is {
    #"" -> {
      // Continue
      expect head_list(pends_in) == head_list(pends_out)
      return(total, tail_list(pends_in), tail_list(pends_out))
    }
    _ -> {
      // Expire
      let (amount, timeout, _) = head_list(pends_in)
      expect Finite(lower_bound) = lower_bound
      expect timeout <= lower_bound
      return(total + amount, tail_list(pends_in), pends_out)
    }
  }
}
