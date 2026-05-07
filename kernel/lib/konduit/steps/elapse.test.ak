use aiken/interval.{Finite}
use konduit/steps/elapse
use konduit/tests/fixture as f

// elapse.do: Closed -> consumed (EOL, no output)
// Requires lower_bound >= elapse_at

const elapse_at = f.much_later

const stage_in = f.closed(0, [], elapse_at)

test test_elapse_ok() {
  // lower_bound is after elapse_at
  let lb = Finite(elapse_at + 1)
  let vkey = elapse.do(lb, f.constants, stage_in)
  vkey == f.add_vkey
}

test test_elapse_exact() {
  // lower_bound == elapse_at is also valid
  let lb = Finite(elapse_at)
  let vkey = elapse.do(lb, f.constants, stage_in)
  vkey == f.add_vkey
}

test test_elapse_too_early() fail {
  let lb = Finite(elapse_at - 1)
  expect _ = elapse.do(lb, f.constants, stage_in)
}

test test_elapse_wrong_stage() fail {
  // stage_in must be Closed
  let lb = Finite(elapse_at)
  expect _ = elapse.do(lb, f.constants, f.opened(0, []))
}
