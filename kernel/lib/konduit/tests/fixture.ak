use aiken/collection/list
use aiken/crypto.{ScriptHash, blake2b_224}
use aiken/interval.{Interval}
use cardano/address.{Address, Script}
use cardano/assets
use cardano/transaction.{
  InlineDatum, Input, Output, OutputReference, Spend, Transaction, placeholder,
}
use konduit/types as t

// ── Base constants ────────────────────────────────────────────────────────────

pub const own_hash: ScriptHash =
  #"00000000000000000000000000000000000000000000000000000000"

pub const add_vkey =
  #"1111111111111111111111111111111111111111111111111111111111111111"

pub const sub_vkey =
  #"2222222222222222222222222222222222222222222222222222222222222222"

/// All-zero 64-byte signature — valid under no_crypto env
pub const sig =
  #"00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"

/// All-zero 32-byte secret/lock — valid under no_crypto env
pub const secret =
  #"0000000000000000000000000000000000000000000000000000000000000000"

pub const tag: ByteArray = "test_channel"

pub const close_period = 24 * 60 * 60 * 1000

pub const constants = t.Constants { tag, add_vkey, sub_vkey, close_period }

/// Pre-hashed signatories for extra_signatories lists
pub const add_sig = blake2b_224(add_vkey)

pub const sub_sig = blake2b_224(sub_vkey)

// ── Timestamps (posix ms) ─────────────────────────────────────────────────────

pub const now = 1_765_889_000_000

pub const soon = now + 60_000

pub const later = soon + 60_000

/// Far enough past close_period (48h) to satisfy elapse checks
pub const much_later = now + 48 * 60 * 60 * 1000

// ── Values ────────────────────────────────────────────────────────────────────

pub fn ada(n: Int) -> assets.Value {
  assets.from_lovelace(n * 1_000_000)
}

// ── Stages ────────────────────────────────────────────────────────────────────

pub fn opened(subbed: Int, useds: List<t.Used>) -> t.Stage {
  t.Opened(subbed, useds)
}

pub fn closed(subbed: Int, useds: List<t.Used>, elapse_at: Int) -> t.Stage {
  t.Closed(subbed, useds, elapse_at)
}

pub fn responded(pends_amount: Int, pends: List<t.Pending>) -> t.Stage {
  t.Responded(pends_amount, pends)
}

// ── Datums ────────────────────────────────────────────────────────────────────

pub fn datum(stage: t.Stage) -> t.Datum {
  (own_hash, constants, stage)
}

pub fn datum_with(
  hash: ScriptHash,
  consts: t.Constants,
  stage: t.Stage,
) -> t.Datum {
  (hash, consts, stage)
}

// ── Outputs & Inputs ──────────────────────────────────────────────────────────

fn own_address() -> Address {
  Address(Script(own_hash), None)
}

pub fn channel_output(stage: t.Stage, lovelace: Int) -> Output {
  Output(
    own_address(),
    assets.from_lovelace(lovelace),
    InlineDatum(datum(stage)),
    None,
  )
}

pub fn channel_input(
  oref: OutputReference,
  stage: t.Stage,
  lovelace: Int,
) -> Input {
  Input(oref, channel_output(stage, lovelace))
}

// ── Validity ranges ───────────────────────────────────────────────────────────

/// Upper-bound only: for Add, Sub, Close, Respond, Unlock
pub fn before(upper: Int) -> Interval {
  interval.before(upper)
}

/// Lower-bound only: for Expire, End, Elapse
pub fn after(lower: Int) -> Interval {
  interval.after(lower)
}

/// Both bounds set
pub fn between(lower: Int, upper: Int) -> Interval {
  interval.between(lower, upper)
}

// ── Tx builders ───────────────────────────────────────────────────────────────

/// Single channel, continuing output (Cont steps: Add, Sub, Close, Respond, Unlock, Expire)
pub fn simple_cont_tx(
  oref: OutputReference,
  stage_in: t.Stage,
  lovelace_in: Int,
  step: t.Cont,
  stage_out: t.Stage,
  lovelace_out: Int,
  signatories: List<ByteArray>,
  validity_range: Interval,
) -> Transaction {
  let redeemer: Data = t.Main([t.StepCont(step)])
  Transaction {
    ..placeholder,
    inputs: [channel_input(oref, stage_in, lovelace_in)],
    outputs: [channel_output(stage_out, lovelace_out)],
    extra_signatories: signatories,
    validity_range,
    redeemers: [Pair(Spend(oref), redeemer)],
  }
}

/// Single channel, end-of-life step (End, Elapse — no continuing output)
pub fn simple_eol_tx(
  oref: OutputReference,
  stage_in: t.Stage,
  lovelace_in: Int,
  step: t.Eol,
  signatories: List<ByteArray>,
  validity_range: Interval,
) -> Transaction {
  let redeemer: Data = t.Main([t.StepEol(step)])
  Transaction {
    ..placeholder,
    inputs: [channel_input(oref, stage_in, lovelace_in)],
    outputs: [],
    extra_signatories: signatories,
    validity_range,
    redeemers: [Pair(Spend(oref), redeemer)],
  }
}

/// Mutual spend: both parties sign, no continuing output required
pub fn mutual_tx(
  oref: OutputReference,
  stage_in: t.Stage,
  lovelace_in: Int,
) -> Transaction {
  let redeemer: Data = t.Mutual
  Transaction {
    ..placeholder,
    inputs: [channel_input(oref, stage_in, lovelace_in)],
    outputs: [],
    extra_signatories: [add_sig, sub_sig],
    redeemers: [Pair(Spend(oref), redeemer)],
  }
}

// ── Multi-channel tx ──────────────────────────────────────────────────────────

/// Per-channel spec for multi_cont_tx
pub type ContSpec {
  oref: OutputReference,
  stage_in: t.Stage,
  lovelace_in: Int,
  step: t.Cont,
  stage_out: t.Stage,
  lovelace_out: Int,
}

/// N channels in one tx.
/// First channel drives Main([step_0..step_N-1]); all others carry Defer.
pub fn multi_cont_tx(
  channels: List<ContSpec>,
  signatories: List<ByteArray>,
  validity_range: Interval,
) -> Transaction {
  let steps = list.map(channels, fn(c) { t.StepCont(c.step) })
  let inputs =
    list.map(
      channels,
      fn(c) { channel_input(c.oref, c.stage_in, c.lovelace_in) },
    )
  let outputs =
    list.map(channels, fn(c) { channel_output(c.stage_out, c.lovelace_out) })
  Transaction {
    ..placeholder,
    inputs,
    outputs,
    extra_signatories: signatories,
    validity_range,
    redeemers: build_redeemers(channels, steps),
  }
}

fn build_redeemers(channels: List<ContSpec>, steps: List<t.Step>) {
  when channels is {
    [] -> []
    [first, ..rest] -> {
      let main: Data = t.Main(steps)
      let defer: Data = t.Defer
      [
        Pair(Spend(first.oref), main),
        ..list.map(rest, fn(c) { Pair(Spend(c.oref), defer) })
      ]
    }
  }
}
