use aiken/crypto.{ScriptHash}
use cardano/address.{Address, Credential, Script}
use cardano/transaction.{
  Datum, InlineDatum, Input, Output, OutputReference, Transaction,
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
pub fn handle(datum: t.Datum, own_oref: OutputReference, tx: Transaction) {
  own_oref != main_oref(Script(datum.1st), tx.inputs)
}

pub fn get_own_hash(datum: Datum) {
  expect InlineDatum(inline): Datum = datum
  expect (own_hash, _, _): (ScriptHash, Data, Data) = inline
  own_hash
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
