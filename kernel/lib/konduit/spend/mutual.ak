use aiken/collection/list
use aiken/crypto.{blake2b_224}
use cardano/address.{Address}
use cardano/transaction.{Input, Output, OutputReference, Transaction}
use konduit/types as t

/// A channel can be spent by mutual consent with no further verification on the
/// channel itself. In such cases, it is spent with the `Mutual` redeemer. To prevent
/// exploits, and by the way `Defer` and `Main` interplay, we prevent any other
/// channels spent alongside a `Mutual` spend.
pub fn handle(datum: t.Datum, own_oref: OutputReference, tx: Transaction) {
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
