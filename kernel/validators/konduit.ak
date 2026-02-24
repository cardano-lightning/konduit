use cardano/transaction.{OutputReference, Transaction}
use konduit/spend.{do}
use konduit/types as t

validator konduit {
  spend(
    datum: Option<t.Datum>,
    redeemer: t.Redeemer,
    own_oref: OutputReference,
    tx: Transaction,
  ) {
    trace @"X": @"Start"
    expect Some(datum) = datum
    do(datum, redeemer, own_oref, tx)
  }

  else(_) {
    fail
  }
}
