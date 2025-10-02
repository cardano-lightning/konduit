use cardano/transaction.{OutputReference, Transaction}
use konduit/spend.{handle}
use konduit/types as t

validator konduit {
  spend(
    datum: Option<t.Datum>,
    redeemer: t.Redeemer,
    own_oref: OutputReference,
    tx: Transaction,
  ) {
    expect Some(datum) = datum
    handle(datum, redeemer, own_oref, tx)
  }

  else(_) {
    fail
  }
}
