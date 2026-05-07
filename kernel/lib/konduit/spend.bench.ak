use aiken/builtin
use aiken/collection/list
use aiken/crypto.{sha2_256}
use aiken/fuzz
use cardano/transaction.{OutputReference}
use konduit/spend
use konduit/tests/fixture as f
use konduit/types as t

// ── Helpers ───────────────────────────────────────────────────────────────────

fn make_oref(i: Int) -> OutputReference {
  OutputReference(builtin.integer_to_bytearray(False, 32, i), 0)
}

// Squash claiming zero owed with no cheques processed
fn zero_squash() -> t.Squash {
  ((0, 0, []), f.sig)
}

// ContSpec for a Sub step: Opened → Opened, no lovelace change, no cheques
fn sub_spec(i: Int) -> f.ContSpec {
  f.ContSpec {
    oref: make_oref(i),
    stage_in: f.opened(0, []),
    lovelace_in: 100_000_000,
    step: t.Sub(zero_squash(), []),
    stage_out: f.opened(0, []),
    lovelace_out: 100_000_000,
  }
}

// ContSpec for a Respond step: Closed → Responded, no lovelace change, no cheques
fn respond_spec(i: Int) -> f.ContSpec {
  f.ContSpec {
    oref: make_oref(i),
    stage_in: f.closed(0, [], f.much_later),
    lovelace_in: 100_000_000,
    step: t.Respond(zero_squash(), []),
    stage_out: f.responded(0, []),
    lovelace_out: 100_000_000,
  }
}

// ContSpec for an Unlock step: Responded → Responded, revealing one preimage
fn unlock_spec(i: Int) -> f.ContSpec {
  let hash = sha2_256(f.secret)
  let pend = (1_000_000, f.later, hash)
  f.ContSpec {
    oref: make_oref(i),
    stage_in: f.responded(1_000_000, [pend]),
    lovelace_in: 100_000_000,
    step: t.Unlock([f.secret]),
    stage_out: f.responded(0, []),
    lovelace_out: 100_000_000,
  }
}

fn build_specs(
  start: Int,
  count: Int,
  make: fn(Int) -> f.ContSpec,
) -> List<f.ContSpec> {
  if count <= 0 {
    []
  } else {
    list.map(list.range(start, start + count - 1), make)
  }
}

// ── Sampler ───────────────────────────────────────────────────────────────────

// Aiken passes size = 0, 1, 2, ... → n_channels = 1, 2, 3, ...
fn n_channels(size: Int) -> Fuzzer<Int> {
  fuzz.constant(size + 1)
}

// ── Benchmark ─────────────────────────────────────────────────────────────────

// Mixed tx: mostly Sub, 1 Respond (n>=2), 1 Unlock (n>=3).
// As n grows, Sub dominates: n-2 Sub + 1 Respond + 1 Unlock.
bench bench_mixed(n via n_channels) {
  let n_unlock =
    if n >= 3 {
      1
    } else {
      0
    }
  let n_respond =
    if n >= 2 {
      1
    } else {
      0
    }
  let n_sub = n - n_unlock - n_respond

  let sub_ch = build_specs(0, n_sub, sub_spec)
  let respond_ch = build_specs(n_sub, n_respond, respond_spec)
  let unlock_ch = build_specs(n_sub + n_respond, n_unlock, unlock_spec)

  let channels = list.concat(sub_ch, list.concat(respond_ch, unlock_ch))

  let tx = f.multi_cont_tx(channels, [f.sub_sig], f.before(f.soon))

  expect [first, ..] = channels
  let datum = f.datum(first.stage_in)
  let steps = list.map(channels, fn(c) { t.StepCont(c.step) })
  spend.do(datum, t.Main(steps), first.oref, tx)
}
