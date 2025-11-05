use aiken/crypto.{blake2b_224}
use cardano/address.{Address, Script}
use cardano/assets
use cardano/transaction.{
  InlineDatum, Input, Output, OutputReference, Spend, Transaction, placeholder,
}
use konduit/spend.{do}
use konduit/types as t

test test_one() {
  let b28_0 = #"00000000000000000000000000000000000000000000000000000000"
  // let b32_0 = #"0000000000000000000000000000000000000000000000000000000000000000"
  let b32_1 =
    #"1111111111111111111111111111111111111111111111111111111111111111"
  let b32_2 =
    #"2222222222222222222222222222222222222222222222222222222222222222"
  let b32_3 =
    #"3333333333333333333333333333333333333333333333333333333333333333"

  // let b32_4 = #"4444444444444444444444444444444444444444444444444444444444444444"
  let own_hash = b28_0
  let tag = "my_little_channel"
  let add_vkey = b32_1
  let sub_vkey = b32_2
  let close_period = 24 * 60 * 60 * 1000
  let constants = t.Constants { tag, add_vkey, sub_vkey, close_period }
  let stage = t.Opened(0)

  let datum = (own_hash, constants, stage)
  let step = t.StepCont(t.Add)
  let redeemer = t.Main([step])
  let own_oref = OutputReference(b32_3, 0)

  let own_cred = Script(own_hash)
  let own_address = Address(own_cred, None)

  let ada = 1_000_000
  let own_output =
    Output(
      own_address,
      assets.from_lovelace(100 * ada),
      InlineDatum(datum),
      None,
    )
  let cont_output =
    Output(
      own_address,
      assets.from_lovelace(200 * ada),
      InlineDatum(datum),
      None,
    )

  let inputs = [Input(own_oref, own_output)]

  let outputs = [cont_output]

  let extra_signatories = [blake2b_224(add_vkey)]

  let tx = Transaction { ..placeholder, inputs, outputs, extra_signatories }

  expect do(datum, redeemer, own_oref, tx)
}

test test_two() {
  let b28_0 = #"00000000000000000000000000000000000000000000000000000000"
  let b32_0 =
    #"0000000000000000000000000000000000000000000000000000000000000000"
  let b32_1 =
    #"1111111111111111111111111111111111111111111111111111111111111111"
  let b32_2 =
    #"2222222222222222222222222222222222222222222222222222222222222222"
  let b32_3 =
    #"3333333333333333333333333333333333333333333333333333333333333333"
  let b32_4 =
    #"4444444444444444444444444444444444444444444444444444444444444444"

  let ada = 1_000_000

  let own_hash = b28_0
  let own_cred = Script(own_hash)
  let own_address = Address(own_cred, None)

  let close_period = 24 * 60 * 60 * 1000

  let tag0 = "my_little_channel"
  let add_vkey0 = b32_1
  let sub_vkey0 = b32_2
  let constants0 =
    t.Constants {
      tag: tag0,
      add_vkey: add_vkey0,
      sub_vkey: sub_vkey0,
      close_period,
    }
  let stage0 = t.Opened(0)
  let datum0 = (own_hash, constants0, stage0)
  let step0 = t.StepCont(t.Add)
  let own_oref0 = OutputReference(b32_0, 0)

  let own_output0 =
    Output(
      own_address,
      assets.from_lovelace(100 * ada),
      InlineDatum(datum0),
      None,
    )
  let cont_output0 =
    Output(
      own_address,
      assets.from_lovelace(200 * ada),
      InlineDatum(datum0),
      None,
    )

  let tag1 = "their_little_channel"
  let add_vkey1 = b32_3
  let sub_vkey1 = b32_4
  let constants1 =
    t.Constants {
      tag: tag1,
      add_vkey: add_vkey1,
      sub_vkey: sub_vkey1,
      close_period,
    }
  let stage1 = t.Opened(0)
  let datum1 = (own_hash, constants1, stage1)
  let step1 = t.StepCont(t.Add)
  let own_oref1 = OutputReference(b32_1, 0)

  let own_output1 =
    Output(
      own_address,
      assets.from_lovelace(100 * ada),
      InlineDatum(datum1),
      None,
    )
  let cont_output1 =
    Output(
      own_address,
      assets.from_lovelace(200 * ada),
      InlineDatum(datum1),
      None,
    )

  let redeemer0 = t.Main([step0, step1])
  let redeemer0_data: Data = redeemer0
  let redeemer1 = t.Defer
  let redeemer1_data: Data = redeemer1
  let redeemers =
    [
      Pair(Spend(own_oref0), redeemer0_data),
      Pair(Spend(own_oref1), redeemer1_data),
    ]

  let inputs = [Input(own_oref0, own_output0), Input(own_oref1, own_output1)]

  let outputs = [cont_output0, cont_output1]

  let extra_signatories = [blake2b_224(add_vkey0), blake2b_224(add_vkey1)]

  let tx =
    Transaction { ..placeholder, inputs, outputs, redeemers, extra_signatories }

  expect do(datum0, redeemer0, own_oref0, tx)
}
