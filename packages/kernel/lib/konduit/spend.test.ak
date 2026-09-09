use aiken/crypto.{VerificationKey, blake2b_224}
use aiken/interval.{Interval, between, entirely_before, everything}
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
  let b32_1 = #"1111111111111111111111111111111111111111111111111111111111111111"
  let b32_2 = #"2222222222222222222222222222222222222222222222222222222222222222"
  let b32_3 = #"3333333333333333333333333333333333333333333333333333333333333333"

  // let b32_4 = #"4444444444444444444444444444444444444444444444444444444444444444"
  let own_hash = b28_0
  let tag = "my_little_channel"
  let add_vkey = b32_1
  let sub_vkey = b32_2
  let close_period = 24 * 60 * 60 * 1000
  let constants = (tag, add_vkey, sub_vkey, close_period, t.Ada)
  let stage = t.Opened(0, [])

  let datum = (own_hash, constants, stage)
  let step = t.StepCont(t.Add)
  let redeemer = t.Main([step])
  let own_oref = OutputReference(b32_3, 0)

  let own_cred = Script(own_hash)
  let own_address = Address(own_cred, None)

  let ada = 1_000_000
  let own_output = Output(
    own_address,
    assets.from_lovelace(100 * ada),
    InlineDatum(datum),
    None,
  )
  let cont_output = Output(
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
  let b32_0 = #"0000000000000000000000000000000000000000000000000000000000000000"
  let b32_1 = #"1111111111111111111111111111111111111111111111111111111111111111"
  let b32_2 = #"2222222222222222222222222222222222222222222222222222222222222222"
  let b32_3 = #"3333333333333333333333333333333333333333333333333333333333333333"
  let b32_4 = #"4444444444444444444444444444444444444444444444444444444444444444"

  let ada = 1_000_000

  let own_hash = b28_0
  let own_cred = Script(own_hash)
  let own_address = Address(own_cred, None)

  let close_period = 24 * 60 * 60 * 1000

  let tag0 = "my_little_channel"
  let add_vkey0 = b32_1
  let sub_vkey0 = b32_2
  let constants0 = (tag0, add_vkey0, sub_vkey0, close_period, t.Ada)
  let stage0 = t.Opened(0, [])
  let datum0 = (own_hash, constants0, stage0)
  let step0 = t.StepCont(t.Add)
  let own_oref0 = OutputReference(b32_0, 0)

  let own_output0 = Output(
    own_address,
    assets.from_lovelace(100 * ada),
    InlineDatum(datum0),
    None,
  )
  let cont_output0 = Output(
    own_address,
    assets.from_lovelace(200 * ada),
    InlineDatum(datum0),
    None,
  )

  let tag1 = "their_little_channel"
  let add_vkey1 = b32_3
  let sub_vkey1 = b32_4
  let constants1 = (tag1, add_vkey1, sub_vkey1, close_period, t.Ada)
  let stage1 = t.Opened(0, [])
  let datum1 = (own_hash, constants1, stage1)
  let step1 = t.StepCont(t.Add)
  let own_oref1 = OutputReference(b32_1, 0)

  let own_output1 = Output(
    own_address,
    assets.from_lovelace(100 * ada),
    InlineDatum(datum1),
    None,
  )
  let cont_output1 = Output(
    own_address,
    assets.from_lovelace(200 * ada),
    InlineDatum(datum1),
    None,
  )

  let redeemer0 = t.Main([step0, step1])
  let redeemer0_data: Data = redeemer0
  let redeemer1 = t.Defer
  let redeemer1_data: Data = redeemer1
  let redeemers = [
    Pair(Spend(own_oref0), redeemer0_data),
    Pair(Spend(own_oref1), redeemer1_data),
  ]

  let inputs = [Input(own_oref0, own_output0), Input(own_oref1, own_output1)]

  let outputs = [cont_output0, cont_output1]

  let extra_signatories = [blake2b_224(add_vkey0), blake2b_224(add_vkey1)]

  let tx = Transaction {
    ..placeholder,
    inputs,
    outputs,
    redeemers,
    extra_signatories,
  }

  expect do(datum0, redeemer0, own_oref0, tx)
}

// Deterministic Ed25519 fixtures generated with Python cryptography from seeds
// 01*32 (Consumer/add) and 02*32 (Adaptor/sub). Each signature covers
// cbor.serialise((#"spend-regression-v1", body)); tuples/non-empty lists use
// indefinite CBOR arrays and the empty list uses the definite encoding 0x80.
const fixture_hash = #"01010101010101010101010101010101010101010101010101010101"

const fixture_tag = #"7370656e642d72656772657373696f6e2d7631"

const consumer_vkey =
  #"8a88e3dd7409f195fd52db2d3cba5d72ca6709bf1d94121bf3748801b40f6f5c"

const adaptor_vkey =
  #"8139770ea87d175f56a35466c34c7ecccb8d8a91b4ee37a25df60f5b8fc9b394"

const fixture_close_period = 500

const fixture_deadline = 2_000

const secret_1 =
  #"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"

const lock_1 =
  #"e0e77a507412b120f6ede61f62295b1a7b2ff19d3dcc8f7253e51663470c888e"

const lock_3 =
  #"4ca14526b2751b640d549ce7caf8ac39438592211a0ec370064d57666a682ad6"

const null_squash_signature =
  #"9c7481e3a664b36b7566d80500269be7c640e0b2c6987e742917ca6ad327f5db7ee903f3b992917642cdf578f5f78bb2efa06b88ac3fb78b1d380a26b8b3410d"

const newer_squash_signature =
  #"51a1bff308a903632e50a5644fcc8202455e6e6d270ce61ce92faa0b511474437268d82c45f7ff13343129caf8c7608c1a9027eaf2797338dccc8fc2264d7803"

const excluded_squash_signature =
  #"4d7bd32bbeaa6865ca8096ebae0945284950c62293ff299d122bbe1d0da6aa6c6371f9bf8b6308a55f1fc1b7cc04fb4393dc031aa359599777b4dcf544b0950c"

const cheque_1_signature =
  #"885de4405bce5fed27614672c7257ecb447eab330d27e35c70b6508393cba736430446d863ad300ac0b842ea9ab4db5b9f4af53d6f82b7079eecf099a49bcf02"

const cheque_3_signature =
  #"f36fa76b97fc1b71dd005d6c52f6ee6721eeaeb0dcb65721d4813db80f15f81b7f9464a2a4e25999e33f4e2b3b393e68e6b452fe611504580509417ead3bde09"

const null_squash = ((0, 0, []), null_squash_signature)

const newer_squash = ((200, 2, []), newer_squash_signature)

const excluded_squash = ((50, 2, [1]), excluded_squash_signature)

const unlocked_1 = ((1, 100, 1_200, secret_1), cheque_1_signature)

const locked_1 = ((1, 100, 1_200, lock_1), cheque_1_signature)

const locked_3 = ((3, 50, 3_000, lock_3), cheque_3_signature)

fn fixture_output(stage: t.Stage, amount: Int) -> Output {
  let constants = (
    fixture_tag,
    consumer_vkey,
    adaptor_vkey,
    fixture_close_period,
    t.Ada,
  )
  Output(
    Address(Script(fixture_hash), None),
    assets.from_lovelace(2_000_000 + amount),
    InlineDatum((fixture_hash, constants, stage)),
    None,
  )
}

fn spend_step(
  stage_in: t.Stage,
  amount_in: Int,
  stage_out: t.Stage,
  amount_out: Int,
  step: t.Cont,
  validity_range: Interval,
  signer: VerificationKey,
) {
  let constants = (
    fixture_tag,
    consumer_vkey,
    adaptor_vkey,
    fixture_close_period,
    t.Ada,
  )
  let datum = (fixture_hash, constants, stage_in)
  let own_oref = OutputReference(
    #"abababababababababababababababababababababababababababababababab",
    0,
  )
  let inputs = [Input(own_oref, fixture_output(stage_in, amount_in))]
  let outputs = [fixture_output(stage_out, amount_out)]
  let extra_signatories = [blake2b_224(signer)]
  let tx = Transaction {
    ..placeholder,
    inputs,
    outputs,
    validity_range,
    extra_signatories,
  }
  do(datum, t.Main([t.StepCont(step)]), own_oref, tx)
}

fn respond(
  useds: List<t.Used>,
  subbed: Int,
  amount_in: Int,
  amount_out: Int,
  squash: t.Squash,
  cheques: List<t.Cheque>,
  p_total: Int,
  pendings: List<t.Pending>,
  validity_range: Interval,
) {
  spend_step(
    t.Closed(subbed, useds, fixture_deadline),
    amount_in,
    t.Responded(p_total, pendings),
    amount_out,
    t.Respond(squash, cheques),
    validity_range,
    adaptor_vkey,
  )
}

fn after_expiry_until_deadline() -> Interval {
  let Interval { lower_bound, .. } = between(1_300, fixture_deadline)
  let Interval { upper_bound, .. } = entirely_before(fixture_deadline)
  Interval { lower_bound, upper_bound }
}

test sub_close_respond_uses_expired_recorded_credit() {
  expect
    spend_step(
      t.Opened(0, []),
      500,
      t.Opened(50, [(1, 100)]),
      450,
      t.Sub(null_squash, [unlocked_1]),
      entirely_before(1_100),
      adaptor_vkey,
    )
  expect
    spend_step(
      t.Opened(50, [(1, 100)]),
      450,
      t.Closed(50, [(1, 100)], fixture_deadline),
      450,
      t.Close,
      entirely_before(1_500),
      consumer_vkey,
    )
  respond(
    [(1, 100)],
    50,
    450,
    400,
    null_squash,
    [],
    0,
    [],
    after_expiry_until_deadline(),
  )
}

test respond_counts_used_after_final_locked_cheque() {
  respond(
    [(1, 100), (4, 40)],
    0,
    500,
    360,
    null_squash,
    [t.LockedCheque(locked_3.1st, locked_3.2nd)],
    50,
    [(50, 3_000, lock_3)],
    entirely_before(fixture_deadline),
  )
}

test newer_squash_covers_used_and_preserves_later_pending() {
  respond(
    [(1, 100)],
    0,
    500,
    300,
    newer_squash,
    [t.LockedCheque(locked_3.1st, locked_3.2nd)],
    50,
    [(50, 3_000, lock_3)],
    entirely_before(fixture_deadline),
  )
}

test squash_exclusion_retains_recorded_credit() {
  respond(
    [(1, 100)],
    0,
    500,
    350,
    excluded_squash,
    [],
    0,
    [],
    entirely_before(fixture_deadline),
  )
}

test duplicate_new_cheque_and_used_is_rejected() fail {
  respond(
    [(1, 100)],
    0,
    500,
    400,
    null_squash,
    [t.UnlockedCheque(unlocked_1.1st, unlocked_1.2nd)],
    0,
    [],
    entirely_before(1_100),
  )
}

test cheque_already_covered_by_squash_is_rejected() fail {
  respond(
    [],
    0,
    500,
    300,
    newer_squash,
    [t.LockedCheque(locked_1.1st, locked_1.2nd)],
    100,
    [(100, 1_200, lock_1)],
    entirely_before(1_100),
  )
}

test incorrect_pending_total_is_rejected() fail {
  respond(
    [],
    0,
    500,
    500,
    null_squash,
    [t.LockedCheque(locked_3.1st, locked_3.2nd)],
    49,
    [(50, 3_000, lock_3)],
    entirely_before(fixture_deadline),
  )
}

test incorrect_pending_list_is_rejected() fail {
  respond(
    [],
    0,
    500,
    500,
    null_squash,
    [t.LockedCheque(locked_3.1st, locked_3.2nd)],
    50,
    [],
    entirely_before(fixture_deadline),
  )
}

test squash_covered_used_cannot_increase_withdrawal() fail {
  respond(
    [(1, 100)],
    0,
    500,
    299,
    newer_squash,
    [t.LockedCheque(locked_3.1st, locked_3.2nd)],
    50,
    [(50, 3_000, lock_3)],
    entirely_before(fixture_deadline),
  )
}

test finite_response_upper_before_and_at_deadline_is_accepted() {
  expect
    respond(
      [],
      0,
      500,
      300,
      newer_squash,
      [],
      0,
      [],
      entirely_before(fixture_deadline - 1),
    )
  respond(
    [],
    0,
    500,
    300,
    newer_squash,
    [],
    0,
    [],
    entirely_before(fixture_deadline),
  )
}

test squash_only_response_after_deadline_is_rejected() fail {
  respond(
    [],
    0,
    500,
    300,
    newer_squash,
    [],
    0,
    [],
    entirely_before(fixture_deadline + 1),
  )
}

test squash_only_response_without_finite_upper_is_rejected() fail {
  respond([], 0, 500, 300, newer_squash, [], 0, [], everything)
}

test locked_only_response_after_deadline_is_rejected() fail {
  respond(
    [],
    0,
    500,
    500,
    null_squash,
    [t.LockedCheque(locked_3.1st, locked_3.2nd)],
    50,
    [(50, 3_000, lock_3)],
    entirely_before(fixture_deadline + 1),
  )
}

test locked_only_response_without_finite_upper_is_rejected() fail {
  respond(
    [],
    0,
    500,
    500,
    null_squash,
    [t.LockedCheque(locked_3.1st, locked_3.2nd)],
    50,
    [(50, 3_000, lock_3)],
    everything,
  )
}

test tampered_squash_evidence_is_rejected() fail {
  let tampered_squash = ((201, 2, []), newer_squash_signature)
  respond(
    [],
    0,
    500,
    299,
    tampered_squash,
    [],
    0,
    [],
    entirely_before(fixture_deadline),
  )
}

test consumer_cannot_authorize_response() fail {
  spend_step(
    t.Closed(0, [], fixture_deadline),
    500,
    t.Responded(0, []),
    300,
    t.Respond(newer_squash, []),
    entirely_before(fixture_deadline),
    consumer_vkey,
  )
}
