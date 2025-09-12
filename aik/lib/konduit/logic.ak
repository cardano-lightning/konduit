use aiken/collection/list.{foldl2}
use aiken/crypto.{ScriptHash, VerificationKey, VerificationKeyHash, blake2b_224}
use aiken/interval.{Finite, Interval, IntervalBound}
use cardano/address.{Address, Credential, Script}
use cardano/assets.{Value}
use cardano/transaction.{
  Datum, InlineDatum, Input, Output, OutputReference, Transaction,
}
use extra/xassets.{only_ada}
use konduit/prelude.{Amount, Index, Timestamp}
use konduit/types as t
use konduit/wellformed.{
  verify_expire, verify_m_receipt, verify_receipt, verify_secrets,
}

/// Based on https://github.com/kompact-io/subbit-xyz
type Signers =
  List<VerificationKeyHash>

fn signed_by(vkhs: Signers, vkey: VerificationKey) {
  vkhs |> list.has(vkey |> blake2b_224)
}

type Bound =
  IntervalBound<Int>

fn unbound(b: Bound) {
  expect IntervalBound { bound_type: Finite(b), .. } = b
  b
}

fn unbounds(
  validity_range: Interval<Int>,
  returns: fn(Timestamp, Timestamp) -> result,
) -> result {
  let Interval { lower_bound, upper_bound } = validity_range
  returns(unbound(lower_bound), unbound(upper_bound))
}

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
  let lower_bound, upper_bound <- unbounds(validity_range)
  let (own_hash, _, _) = datum
  let with =
    mk_with(own_hash, extra_signatories, lower_bound, upper_bound, _, _, _, _)
  let inputs, _ <-
    foldl2(steps, inputs, outputs, with, _)
  inputs == []
}

fn mk_with(
  own_hash: ScriptHash,
  signers: Signers,
  upper_bound: Timestamp,
  lower_bound: Timestamp,
  step: t.Step,
  inputs: List<Input>,
  outputs: List<Output>,
  return: fn(List<Input>, List<Output>) -> result,
) -> result {
  let
    address_in,
    value_in,
    constants,
    stage_in,
    inputs,
  <- yield_in(own_hash, inputs)
  when step is {
    t.StepCont(step) -> {
      let
        value_out,
        stage_out,
        outputs,
      <- yield_out(address_in, constants, outputs)
      expect
        when step is {
          t.Add ->
            do_add(signers, constants, value_in, stage_in, value_out, stage_out)
          t.Sub(squash, unlockeds) ->
            do_sub(
              signers,
              upper_bound,
              constants,
              value_in,
              stage_in,
              value_out,
              stage_out,
              squash,
              unlockeds,
            )
          t.Close ->
            do_close(
              signers,
              upper_bound,
              constants,
              value_in,
              stage_in,
              value_out,
              stage_out,
            )
          t.Respond(squash, mixs) ->
            do_respond(
              signers,
              upper_bound,
              constants,
              value_in,
              stage_in,
              value_out,
              stage_out,
              squash,
              mixs,
            )
          t.Unlock(secrets) ->
            do_unlock(
              signers,
              upper_bound,
              constants,
              value_in,
              stage_in,
              value_out,
              stage_out,
              secrets,
            )
          t.Expire(_) ->
            do_expire(
              signers,
              lower_bound,
              constants,
              value_in,
              stage_in,
              value_out,
              stage_out,
            )
        }
      return(inputs, outputs)
    }
    t.StepEol(step) -> {
      expect
        when step is {
          t.End -> do_end(signers, constants, stage_in)
          t.Elapse -> do_elapse(signers, lower_bound, constants, stage_in)
        }
      return(inputs, outputs)
    }
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
  expect extra_signatories |> signed_by(add_vkey)
  expect extra_signatories |> signed_by(sub_vkey)
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

/// #### Amount in
/// 
/// A channel supports mono-assets. See [funds](../adrs/funds.md). The channel
/// currency is recorded in the constants. On a channel input, we are permissive
/// about the value, ie any unanticipated native assets are ignored.
fn amount_in(value: Value) -> Amount {
  value |> assets.lovelace_of
}

/// #### Amount out
/// 
/// Continuing outputs value is handled in a stricter manner than channel inputs. The
/// amount out function is similar to the amount in function, but checks the value
/// contains no additional assets. This is to prevent token spamming attacks.
fn amount_out(value: Value) -> Amount {
  only_ada(value)
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

pub fn do_add(
  signers: Signers,
  constants: t.Constants,
  value_in: Value,
  stage_in: t.Stage,
  value_out: Value,
  stage_out: t.Stage,
) -> Bool {
  let t.Constants { add_vkey, .. } = constants
  expect t.Opened(_) = stage_in
  expect stage_in == stage_out
  expect signers |> signed_by(add_vkey)
  amount_in(value_in) < amount_out(value_out)
}

pub fn do_sub(
  signers: Signers,
  upper_bound: Timestamp,
  constants: t.Constants,
  value_in: Value,
  stage_in: t.Stage,
  value_out: Value,
  stage_out: t.Stage,
  squash: t.Squash,
  unlockeds: List<t.Unlocked>,
) -> Bool {
  let t.Constants { tag, add_vkey, sub_vkey, .. } = constants
  expect t.Opened(subbed_in) = stage_in
  expect t.Opened(subbed_out) = stage_out
  expect subbed_out - subbed_in == amount_in(value_in) - amount_out(value_out)
  let amount = verify_receipt(upper_bound, tag, add_vkey, squash, unlockeds)
  expect signers |> signed_by(sub_vkey)
  amount >= subbed_out
}

pub fn do_close(
  signers: Signers,
  upper_bound: Timestamp,
  constants: t.Constants,
  value_in: Value,
  stage_in: t.Stage,
  value_out: Value,
  stage_out: t.Stage,
) -> Bool {
  let t.Constants { add_vkey, close_period, .. } = constants
  expect t.Opened(subbed) = stage_in
  expect t.Closed(subbed_, elapse_at) = stage_out
  expect subbed == subbed_
  expect amount_in(value_in) <= amount_out(value_out)
  expect signers |> signed_by(add_vkey)
  elapse_at >= upper_bound + close_period
}

pub fn do_respond(
  signers: Signers,
  upper_bound: Timestamp,
  constants: t.Constants,
  value_in: Value,
  stage_in: t.Stage,
  value_out: Value,
  stage_out: t.Stage,
  squash: t.Squash,
  mixs: List<t.Mix>,
) -> Bool {
  let t.Constants { tag, add_vkey, sub_vkey, .. } = constants
  expect t.Closed(subbed_in, _) = stage_in
  expect t.Responded(subbed_out, pends_amount, pends) = stage_out
  expect subbed_out - subbed_in == amount_in(value_in) - amount_out(value_out)
  let amount =
    verify_m_receipt(
      upper_bound,
      tag,
      add_vkey,
      squash,
      mixs,
      pends_amount,
      pends,
    )
  expect signers |> signed_by(sub_vkey)
  amount >= subbed_out
}

pub fn do_unlock(
  signers: Signers,
  upper_bound: Timestamp,
  constants: t.Constants,
  value_in: Value,
  stage_in: t.Stage,
  value_out: Value,
  stage_out: t.Stage,
  secrets: List<(Index, t.Secret)>,
) -> Bool {
// FIXME
  let t.Constants { tag, sub_vkey, add_vkey, .. } = constants
  expect t.Responded(subbed_in, pends_amount_in, pends_in) = stage_in
  expect t.Responded(subbed_out, pends_amount_out, pends_out) = stage_out
  let amount =
    verify_secrets(
      upper_bound,
      tag,
      add_vkey,
      pends_amount_in,
      pends_in,
      pends_amount_out,
      pends_out,
      secrets,
    )
  expect signers |> signed_by(sub_vkey)
  amount >= subbed_out
}

pub fn do_expire(
  signers: Signers,
  lower_bound: Timestamp,
  constants: t.Constants,
  value_in: Value,
  stage_in: t.Stage,
  value_out: Value,
  stage_out: t.Stage,
) -> Bool {
// FIXME
  let t.Constants { tag, add_vkey, .. } = constants
  expect t.Responded(subbed_in, pends_amount_in, pends_in) = stage_in
  expect t.Responded(subbed_out, pends_amount_out, pends_out) = stage_out
  expect subbed_in == subbed_out
  let amount =
    verify_expire(
      lower_bound,
      tag,
      add_vkey,
      pends_amount_in,
      pends_in,
      pends_amount_out,
      pends_out,
    )
  expect signers |> signed_by(add_vkey)
  False
}

pub fn do_end(
  signers: Signers,
  constants: t.Constants,
  stage_in: t.Stage,
) -> Bool {
  let t.Constants { add_vkey, .. } = constants
  expect t.Responded(_, pends_amount, _) = stage_in
  expect pends_amount == 0
  signers |> signed_by(add_vkey)
}

pub fn do_elapse(
  signers: Signers,
  lower_bound: Timestamp,
  constants: t.Constants,
  stage_in: t.Stage,
) -> Bool {
  let t.Constants { add_vkey, .. } = constants
  expect t.Closed(_, expire_at) = stage_in
  expect signers |> signed_by(add_vkey)
  expire_at <= lower_bound
}

fn coerce_datum(d: Datum) -> t.Datum {
  expect InlineDatum(data) = d
  expect d: t.Datum = data
  d
}
