use aiken/collection/list
use aiken/crypto.{ScriptHash, VerificationKey, blake2b_224}
use cardano/address.{Address, Script}
use cardano/transaction.{Input, Output, Transaction}
use extra/xlist.{foldl5}
use extra/xoption
use extra/xtime
use konduit/io.{yield_in, yield_out}
use konduit/prelude.{Timestamp}
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
  let Transaction { inputs, outputs, validity_range, extra_signatories, .. } =
    tx
  let (own_hash, _, _) = datum
  let with =
    mk_with(own_hash, _, _, _, _, _, _, _)
  let inputs, _outputs, vkeys, lower_bound, upper_bound <-
    foldl5(steps, inputs, outputs, [], None, None, with, _)
  expect
    vkeys
      |> list.all(
          fn(vkey) { extra_signatories |> list.has(vkey |> blake2b_224) },
        )
  expect
    when lower_bound is {
      None -> True
      Some(b) -> b < xtime.unbound(validity_range.lower_bound)
    }
  expect
    when upper_bound is {
      None -> True
      Some(b) -> b > xtime.unbound(validity_range.upper_bound)
    }
  let own_cred = Script(own_hash)
  inputs
    |> list.all(fn(i) { i.output.address.payment_credential != own_cred })
}

type WithReturn<result> =
  fn(
    List<Input>,
    List<Output>,
    List<VerificationKey>,
    Option<Timestamp>,
    Option<Timestamp>,
  ) ->
    result

fn mk_with(
  own_hash: ScriptHash,
  step: t.Step,
  inputs: List<Input>,
  outputs: List<Output>,
  vkeys: List<VerificationKey>,
  lower_bound: Option<Timestamp>,
  upper_bound: Option<Timestamp>,
  return: WithReturn<result>,
) -> result {
  let address, v_in, c, s_in, inputs <- yield_in(own_hash, inputs)
  when step is {
    t.StepCont(step) -> {
      let v_out, s_out, outputs <- yield_out(address, c, outputs)
      let vkey, lower_bound_, upper_bound_ <-
        when step is {
          t.Add ->
            add.do(c, v_in, s_in, v_out, s_out, _)
          t.Sub(squash, more) ->
            sub.do(c, v_in, s_in, v_out, s_out, squash, more, _)
          t.Close ->
            close.do(c, v_in, s_in, v_out, s_out, _)
          t.Respond(squash, more) ->
            respond.do(c, v_in, s_in, v_out, s_out, squash, more, _)
          t.Unlock(unpend) ->
            unlock.do(c, v_in, s_in, v_out, s_out, unpend, _)
          t.Expire(unpend) ->
            expire.do(c, s_in, v_out, s_out, unpend, _)
        }
      let lower_bound = xoption.max2(lower_bound, lower_bound_)
      let upper_bound = xoption.min2(upper_bound, upper_bound_)
      let vkeys = vkeys |> xlist.ordered_insert(vkey)
      return(inputs, outputs, vkeys, lower_bound, upper_bound)
    }
    t.StepEol(step) -> {
      let vkey, lower_bound_, _upper_bound <-
        when step is {
          t.End ->
            end.do(c, s_in, _)
          t.Elapse ->
            elapse.do(c, s_in, _)
        }
      let lower_bound = xoption.max2(lower_bound, lower_bound_)
      let vkeys = vkeys |> xlist.ordered_insert(vkey)
      return(inputs, outputs, vkeys, lower_bound, upper_bound)
    }
  }
}
