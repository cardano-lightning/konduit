use aiken/collection/list
use aiken/crypto.{ScriptHash, VerificationKey, blake2b_224}
use aiken/interval.{Finite, Interval, IntervalBound}
use aiken/primitive/bytearray
use cardano/address.{Address, Credential, Script}
use cardano/assets.{Value}
use cardano/transaction.{
  Datum, InlineDatum, Input, Output, OutputReference, Transaction,
}
use extra/xlist.{foldl5}
use extra/xoption
use konduit/prelude.{Timestamp}
use konduit/steps.{
  do_add, do_close, do_elapse, do_end, do_expire, do_respond, do_sub, do_unlock,
}
use konduit/types as t

/// Based on https://github.com/kompact-io/subbit-xyz
/// ### Batch
/// 
/// In a standard tx, all but one of the script inputs is spent with `Batch`. More
/// precisely, the lexicographical first validator input in the tx inputs must have
/// redeemer `Main`, and all the rest are spent with `Batch`.
/// 
/// The logic when the redeemer is `Batch`:
pub fn get_own_hash(datum: Datum) {
  expect InlineDatum(inline): Datum = datum
  expect (own_hash, _, _): (ScriptHash, Data, Data) = inline
  own_hash
}

pub fn batch(datum: t.Datum, own_oref: OutputReference, tx: Transaction) {
  own_oref != main_oref(Script(datum.1st), tx.inputs)
}

pub fn main_oref(cred: Credential, inputs: List<Input>) -> OutputReference {
  when inputs is {
    [input, ..inputs] -> {
      let Input { output_reference, output: Output { address, datum, .. } } =
        input
      if address.payment_credential == cred {
        expect Script(get_own_hash(datum)) == cred
        output_reference
      } else {
        main_oref(cred, inputs)
      }
    }
    _ -> fail @"impossible"
  }
}

/// ### Main
/// 
/// In a standard tx, the "main" invocation does all the verification steps for all
/// channels.
/// 
/// The logic when the redeemer is `Main(steps)`
/// 
pub fn main(datum: t.Datum, steps: t.Steps, tx: Transaction) {
  trace @"main.0": @"Extract `bounds`, and `extra_signatories`,"
  let Transaction { inputs, outputs, validity_range, extra_signatories, .. } =
    tx
  expect lower_bound = validity
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
      Some(lower_bound) -> lower_bound < unbound(validity_range.lower_bound)
    }
  expect
    when upper_bound is {
      None -> True
      Some(upper_bound) -> upper_bound > unbound(validity_range.upper_bound)
    }
  inputs == []
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
            do_add(c, v_in, s_in, v_out, s_out, _)
          t.Sub(squash, unlockeds) ->
            do_sub(c, v_in, s_in, v_out, s_out, squash, unlockeds, _)
          t.Close ->
            do_close(c, v_in, s_in, v_out, s_out, _)
          t.Respond(squash, mixs) ->
            do_respond(c, v_in, s_in, v_out, s_out, squash, mixs, _)
          t.Unlock(unpend) ->
            do_unlock(c, v_in, s_in, v_out, s_out, unpend, _)
          t.Expire(unpend) ->
            do_expire(c, s_in, v_out, s_out, unpend, _)
        }
      let lower_bound = xoption.max2(lower_bound, lower_bound_)
      let upper_bound = xoption.min2(upper_bound, upper_bound_)
      let vkeys = vkeys |> insert(vkey)
      return(inputs, outputs, vkeys, lower_bound, upper_bound)
    }
    t.StepEol(step) -> {
      let vkey, lower_bound_, _upper_bound <-
        when step is {
          t.End ->
            do_end(c, s_in, _)
          t.Elapse ->
            do_elapse(c, s_in, _)
        }
      let lower_bound = xoption.max2(lower_bound, lower_bound_)
      let vkeys = vkeys |> insert(vkey)
      return(inputs, outputs, vkeys, lower_bound, upper_bound)
    }
  }
}

fn insert(xs: List<ByteArray>, y: ByteArray) {
  when xs is {
    [x, ..xs] ->
      when bytearray.compare(x, y) is {
        Less -> [x, ..insert(xs, y)]
        Equal -> [x, ..xs]
        Greater -> fail @"Impossible"
      }
    _ -> [y]
  }
}

/// ### Mutual
/// 
/// A channel can be spent by mutual consent with no further verification on the
/// channel itself. In such cases, it is spent with the `Mutual` redeemer. To prevent
/// exploits, and by the way `Batch` and `Main` interplay, we prevent any other
/// channels spent alongside a `Mutual` spend.
pub fn mutual(datum: t.Datum, own_oref: OutputReference, tx: Transaction) {
  let (_, constants, _) = datum
  let t.Constants { add_vkey, sub_vkey, .. } = constants
  let Transaction { inputs, extra_signatories, .. } = tx
  trace @"mutual.0": @"Both participants sign"
  expect extra_signatories |> list.has(add_vkey |> blake2b_224)
  expect extra_signatories |> list.has(sub_vkey |> blake2b_224)
  expect Some(own_input) =
    inputs |> list.find(fn(i) { i.output_reference == own_oref })
  let own_cred = own_input.output.address.payment_credential
  trace @"mutual.1": @"Is solo channel input"
  1 == (
    inputs
      |> list.count(fn(i) { i.output.address.payment_credential == own_cred })
  )
}

/// #### Yield channel in
/// 
/// We define a function that "yields" the next channel input. A channel input is
/// identified by its payment credential. The rest of the input is then parsed to
/// check correctness. 
pub type YeildIn<result> =
  fn(Address, Value, t.Constants, t.Stage, List<Input>) -> result

fn yield_in(
  own_hash: ScriptHash,
  inputs: List<Input>,
  return: YeildIn<result>,
) -> result {
  when inputs is {
    [input, ..inputs] -> {
      let Output { address, value, datum, .. } = input.output
      if address.payment_credential == Script(own_hash) {
        let (own_hash_, constants, stage) = coerce_datum(datum)
        expect own_hash_ == own_hash
        return(address, value, constants, stage, inputs)
      } else {
        yield_in(own_hash, inputs, return)
      }
    }
    [] -> fail @"expect channel in"
  }
}

/// #### Yield continuing output
/// 
/// For any non-terminal step there is a continuing output. The checks required are
/// subtly different to that of channel in.
/// 
/// Recall that the continuing output must match address, not just payment
/// credential. Since the currency should be unchanged, we use the currency value
/// from the channel input. The step logic will verify all the entire constant value
/// is unchanged, where relevant.
pub type YeildOut<result> =
  fn(Value, t.Stage, List<Output>) -> result

fn yield_out(
  address_in: Address,
  constants: t.Constants,
  outputs: List<Output>,
  return: YeildOut<result>,
) -> result {
  when outputs is {
    [output, ..outputs] -> {
      let Output { address, value, datum, reference_script } = output
      if address_in == address {
        expect Script(own_hash) = address.payment_credential
        let (own_hash_, constants_, stage) = coerce_datum(datum)
        expect own_hash_ == own_hash
        expect constants_ == constants
        expect reference_script == None
        return(value, stage, outputs)
      } else {
        yield_out(address_in, constants, outputs, return)
      }
    }
    _ -> fail @"expected output"
  }
}

fn coerce_datum(d: Datum) -> t.Datum {
  expect InlineDatum(data) = d
  expect d: t.Datum = data
  d
}

fn unbound(b: IntervalBound<Int>) {
  expect IntervalBound { bound_type: Finite(b), .. } = b
  b
}
