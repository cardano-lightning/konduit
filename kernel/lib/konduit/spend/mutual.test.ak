use cardano/transaction.{Input, OutputReference, Transaction, placeholder}
use konduit/spend/mutual
use konduit/tests/fixture as f

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

const stage = f.opened(0, [])

fn mk_tx(inputs: List<Input>, signatories: List<ByteArray>) -> Transaction {
  Transaction { ..placeholder, inputs, extra_signatories: signatories }
}

test test_mutual_ok() {
  let input = f.channel_input(oref_0, stage, 100_000_000)
  let datum = f.datum(stage)
  let tx = mk_tx([input], [f.add_sig, f.sub_sig])
  mutual.handle(datum, oref_0, tx)
}

test test_mutual_missing_add_sig() fail {
  let input = f.channel_input(oref_0, stage, 100_000_000)
  let datum = f.datum(stage)
  let tx = mk_tx([input], [f.sub_sig])
  mutual.handle(datum, oref_0, tx)
}

test test_mutual_missing_sub_sig() fail {
  let input = f.channel_input(oref_0, stage, 100_000_000)
  let datum = f.datum(stage)
  let tx = mk_tx([input], [f.add_sig])
  mutual.handle(datum, oref_0, tx)
}

test test_mutual_multiple_channel_inputs() fail {
  // Two channel inputs with the same credential: mutual requires exactly one
  let input_0 = f.channel_input(oref_0, stage, 100_000_000)
  let input_1 = f.channel_input(oref_1, stage, 100_000_000)
  let datum = f.datum(stage)
  let tx = mk_tx([input_0, input_1], [f.add_sig, f.sub_sig])
  mutual.handle(datum, oref_0, tx)
}
