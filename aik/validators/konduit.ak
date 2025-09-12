use cardano/transaction.{OutputReference, Transaction}
use konduit/logic.{batch, main, mutual}
use konduit/types as t

validator konduit {
  spend(
    datum: Option<t.Datum>,
    redeemer: t.Redeemer,
    own_oref: OutputReference,
    tx: Transaction,
  ) {
    expect Some(datum) = datum
    when redeemer is {
      t.Batch -> batch(datum, own_oref, tx)
      t.Main(steps) -> main(datum, steps, tx)
      t.Mutual -> mutual(datum, own_oref, tx)
    }
  }

  else(_) {
    fail
  }
}
