use cardano/address.{Address, Script}
use cardano/assets
use cardano/transaction.{InlineDatum, Input, NoDatum, Output, OutputReference}
use konduit/io.{yield_in, yield_out}
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

// A script input with a different hash — used as "other" input to skip
fn other_script_input(oref: OutputReference) -> Input {
  let other_hash = #"ffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
  let addr = Address(Script(other_hash), None)
  let out = Output(addr, assets.from_lovelace(2_000_000), NoDatum, None)
  Input(oref, out)
}

// ── yield_in ──────────────────────────────────────────────────────────────────

test test_yield_in_ok() {
  let stage = f.opened(0, [])
  let input = f.channel_input(oref_0, stage, 100_000_000)
  let addr, value, constants, stage_out, rest <- yield_in(f.own_hash, [input])
  addr == input.output.address && value == input.output.value && constants == f.constants && stage_out == stage && rest == []
}

test test_yield_in_skips_non_script() {
  let stage = f.opened(0, [])
  let channel = f.channel_input(oref_1, stage, 100_000_000)
  let other = other_script_input(oref_0)
  let
    _addr,
    _value,
    _constants,
    stage_out,
    rest,
  <- yield_in(f.own_hash, [other, channel])
  stage_out == stage && rest == []
}

test test_yield_in_returns_remaining() {
  let stage = f.opened(0, [])
  let ch0 = f.channel_input(oref_0, stage, 100_000_000)
  let ch1 = f.channel_input(oref_1, stage, 200_000_000)
  let
    _addr,
    _value,
    _constants,
    _stage,
    rest,
  <- yield_in(f.own_hash, [ch0, ch1])
  rest == [ch1]
}

test test_yield_in_wrong_own_hash() fail {
  let stage = f.opened(0, [])
  let input = f.channel_input(oref_0, stage, 100_000_000)
  let wrong_hash = #"ffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
  // yield_in scans and exhausts inputs without finding wrong_hash -> panic
  yield_in(wrong_hash, [input], fn(_, _, _, _, _) { True })
}

// ── yield_out ─────────────────────────────────────────────────────────────────

test test_yield_out_ok() {
  let stage = f.opened(0, [])
  let input = f.channel_input(oref_0, stage, 100_000_000)
  let output = f.channel_output(stage, 200_000_000)
  let
    value,
    stage_out,
    rest,
  <- yield_out(input.output.address, f.constants, [output])
  value == output.value && stage_out == stage && rest == []
}

test test_yield_out_skips_other_address() {
  let stage = f.opened(0, [])
  let input = f.channel_input(oref_0, stage, 100_000_000)
  let target = f.channel_output(stage, 200_000_000)
  let other_addr =
    Address(
      Script(#"ffffffffffffffffffffffffffffffffffffffffffffffffffffffff"),
      None,
    )
  let other = Output(other_addr, assets.from_lovelace(1_000_000), NoDatum, None)
  let
    _value,
    stage_out,
    rest,
  <- yield_out(input.output.address, f.constants, [other, target])
  stage_out == stage && rest == []
}

test test_yield_out_constants_changed() fail {
  let stage = f.opened(0, [])
  let input = f.channel_input(oref_0, stage, 100_000_000)
  let bad_consts = t.Constants { ..f.constants, tag: "wrong" }
  let bad_datum = f.datum_with(f.own_hash, bad_consts, stage)
  let bad_output =
    Output(
      input.output.address,
      assets.from_lovelace(200_000_000),
      InlineDatum(bad_datum),
      None,
    )
  yield_out(
    input.output.address,
    f.constants,
    [bad_output],
    fn(_, _, _) { True },
  )
}

test test_yield_out_reference_script() fail {
  let stage = f.opened(0, [])
  let input = f.channel_input(oref_0, stage, 100_000_000)
  let bad_output =
    Output(
      input.output.address,
      assets.from_lovelace(200_000_000),
      InlineDatum(f.datum(stage)),
      Some(#"00000000000000000000000000000000000000000000000000000000"),
    )
  yield_out(
    input.output.address,
    f.constants,
    [bad_output],
    fn(_, _, _) { True },
  )
}
