use aiken/collection/list
use aiken/crypto.{ScriptHash, VerificationKey, blake2b_224}
use aiken/interval.{Interval, IntervalBound}
use cardano/address.{Address, Script}
use cardano/transaction.{Input, Output, Transaction}
use extra/xlist.{foldl3}
use konduit/io.{yield_in, yield_out}
use konduit/prelude.{Timebound}
use konduit/steps/add
use konduit/steps/close
use konduit/steps/elapse
use konduit/steps/end
use konduit/steps/expire
use konduit/steps/respond
use konduit/steps/sub
use konduit/steps/unlock
use konduit/types as t

/// In a standard tx, the "main" invocation does all the verification steps for all
/// channels.
pub fn handle(datum: t.Datum, steps: List<t.Step>, tx: Transaction) {
  trace @"M": @"Main"
  let Transaction { inputs, outputs, validity_range, extra_signatories, .. } =
    tx
  let Interval {
    lower_bound: IntervalBound { bound_type: lower_bound, .. },
    upper_bound: IntervalBound { bound_type: upper_bound, .. },
  } = validity_range
  let (own_hash, _, _) = datum
  let with =
    mk_with(own_hash, lower_bound, upper_bound, _, _, _, _, _)
  let inputs, _outputs, vkeys <-
    foldl3(steps, inputs, outputs, [], with, _)
  expect
    vkeys
      |> list.all(
          fn(vkey) { extra_signatories |> list.has(vkey |> blake2b_224) },
        )
  let own_cred = Script(own_hash)
  inputs
    |> list.all(fn(i) { i.output.address.payment_credential != own_cred })
}

type WithReturn<result> =
  fn(List<Input>, List<Output>, List<VerificationKey>) -> result

fn mk_with(
  own_hash: ScriptHash,
  lb: Timebound,
  ub: Timebound,
  step: t.Step,
  inputs: List<Input>,
  outputs: List<Output>,
  vkeys: List<VerificationKey>,
  return: WithReturn<result>,
) -> result {
  let address, v_in, c, s_in, inputs <- yield_in(own_hash, inputs)
  when step is {
    t.StepCont(step) -> {
      let v_out, s_out, outputs <- yield_out(address, c, outputs)
      let vkey =
        when step is {
          t.Add -> add.do(c, v_in, s_in, v_out, s_out)
          t.Sub(squash, more) ->
            sub.do(ub, c, v_in, s_in, v_out, s_out, squash, more)
          t.Close -> close.do(ub, c, v_in, s_in, v_out, s_out)
          t.Respond(squash, more) ->
            respond.do(ub, c, v_in, s_in, v_out, s_out, squash, more)
          t.Unlock(unpend) -> unlock.do(ub, c, v_in, s_in, v_out, s_out, unpend)
          t.Expire(unpend) -> expire.do(lb, c, s_in, v_out, s_out, unpend)
        }
      let vkeys = vkeys |> xlist.ordered_insert(vkey)
      return(inputs, outputs, vkeys)
    }
    t.StepEol(step) -> {
      let vkey =
        when step is {
          t.End -> end.do(lb, c, s_in)
          t.Elapse -> elapse.do(lb, c, s_in)
        }
      let vkeys = vkeys |> xlist.ordered_insert(vkey)
      return(inputs, outputs, vkeys)
    }
  }
}
