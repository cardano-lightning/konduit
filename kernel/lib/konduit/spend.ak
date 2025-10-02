use cardano/transaction.{OutputReference, Transaction}
use konduit/spend/defer
use konduit/spend/main
use konduit/spend/mutual
use konduit/types as t

pub fn handle(
  datum: t.Datum,
  redeemer: t.Redeemer,
  own_oref: OutputReference,
  tx: Transaction,
) {
  when redeemer is {
    t.Defer -> defer.handle(datum, own_oref, tx)
    t.Main(steps) -> main.handle(datum, steps, tx)
    t.Mutual -> mutual.handle(datum, own_oref, tx)
  }
}
