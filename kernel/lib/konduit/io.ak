use aiken/crypto.{ScriptHash}
use cardano/address.{Address, Script}
use cardano/assets.{Value}
use cardano/transaction.{Datum, InlineDatum, Input, Output}
use konduit/types as t

/// #### Yield channel in
/// 
/// We define a function that "yields" the next channel input. A channel input is
/// identified by its payment credential. The rest of the input is then parsed to
/// check correctness. 
pub type YeildIn<result> =
  fn(Address, Value, t.Constants, t.Stage, List<Input>) -> result

pub fn yield_in(
  own_hash: ScriptHash,
  inputs: List<Input>,
  return: YeildIn<result>,
) -> result {
  expect [input, ..inputs] = inputs
  let Output { address, value, datum, .. } = input.output
  if address.payment_credential == Script(own_hash) {
    let (own_hash_, constants, stage) = coerce_datum(datum)
    expect own_hash_ == own_hash
    return(address, value, constants, stage, inputs)
  } else {
    yield_in(own_hash, inputs, return)
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

pub fn yield_out(
  address_in: Address,
  constants: t.Constants,
  outputs: List<Output>,
  return: YeildOut<result>,
) -> result {
  expect [output, ..outputs] = outputs
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

fn coerce_datum(d: Datum) -> t.Datum {
  expect InlineDatum(data) = d
  expect d: t.Datum = data
  d
}
