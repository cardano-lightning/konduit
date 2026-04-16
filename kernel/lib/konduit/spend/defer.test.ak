use cardano/transaction.{OutputReference, Spend, Transaction, placeholder}
use konduit/spend/defer
use konduit/tests/fixture as f
use konduit/types as t

const oref_0 =
  OutputReference(
    #"0000000000000000000000000000000000000000000000000000000000000000",
    0,
  )

const oref_1 =
  OutputReference(
    #"1111111111111111111111111111111111111111111111111111111111111111",
    0,
  )

// defer.handle returns True when own_oref is NOT the first script input
// (i.e. this channel is correctly deferring to another Main input).

test test_defer_is_secondary() {
  // oref_0 is first script input (Main), oref_1 is secondary (Defer)
  let stage = f.opened(0, [])
  let input_0 = f.channel_input(oref_0, stage, 100_000_000)
  let input_1 = f.channel_input(oref_1, stage, 100_000_000)
  let datum_1 = f.datum(stage)
  let redeemer_0: Data = t.Main([t.StepCont(t.Add)])
  let redeemer_1: Data = t.Defer
  let tx =
    Transaction {
      ..placeholder,
      inputs: [input_0, input_1],
      redeemers: [
        Pair(Spend(oref_0), redeemer_0),
        Pair(Spend(oref_1), redeemer_1),
      ],
    }
  defer.handle(datum_1, oref_1, tx)
}

test test_defer_is_primary() fail {
  // oref_0 tries to Defer but IS the first script input — should fail
  let stage = f.opened(0, [])
  let input_0 = f.channel_input(oref_0, stage, 100_000_000)
  let datum_0 = f.datum(stage)
  let redeemer: Data = t.Defer
  let tx =
    Transaction {
      ..placeholder,
      inputs: [input_0],
      redeemers: [Pair(Spend(oref_0), redeemer)],
    }
  defer.handle(datum_0, oref_0, tx)
}
