use aiken/interval.{Finite}
use cardano/assets
use konduit/steps/close
use konduit/tests/fixture as f

// close.do takes Timebound (IntervalBoundType), not a full Interval.
// main.ak extracts bound_type from validity_range before calling steps.

const lovelace = 100_000_000

// elapse_at must be at least close_period beyond upper_bound
const elapse_at = f.soon + f.close_period + 1

const stage_in = f.opened(0, [])

const stage_out = f.closed(0, [], elapse_at)

const ub = Finite(f.soon)

test test_close_ok() {
  let vkey =
    close.do(
      ub,
      f.constants,
      assets.from_lovelace(lovelace),
      stage_in,
      assets.from_lovelace(lovelace),
      stage_out,
    )
  vkey == f.add_vkey
}

test test_close_value_increases() {
  // value is allowed to increase
  let vkey =
    close.do(
      ub,
      f.constants,
      assets.from_lovelace(lovelace),
      stage_in,
      assets.from_lovelace(lovelace + 1_000_000),
      stage_out,
    )
  vkey == f.add_vkey
}

test test_close_elapse_too_soon() fail {
  // elapse_at - close_period < upper_bound
  let bad_elapse_at = f.soon + f.close_period - 1
  expect _ =
    close.do(
      ub,
      f.constants,
      assets.from_lovelace(lovelace),
      stage_in,
      assets.from_lovelace(lovelace),
      f.closed(0, [], bad_elapse_at),
    )
}

test test_close_amount_decreased() fail {
  expect _ =
    close.do(
      ub,
      f.constants,
      assets.from_lovelace(lovelace),
      stage_in,
      assets.from_lovelace(lovelace - 1),
      stage_out,
    )
}

test test_close_subbed_changed() fail {
  // subbed in out datum must match stage_in
  expect _ =
    close.do(
      ub,
      f.constants,
      assets.from_lovelace(lovelace),
      f.opened(500, []),
      assets.from_lovelace(lovelace),
      f.closed(999, [], elapse_at),
    )
}

test test_close_useds_changed() fail {
  // useds in out datum must match stage_in
  expect _ =
    close.do(
      ub,
      f.constants,
      assets.from_lovelace(lovelace),
      f.opened(0, [(1, 100)]),
      assets.from_lovelace(lovelace),
      f.closed(0, [(2, 100)], elapse_at),
    )
}

test test_close_wrong_stage_in() fail {
  // stage_in must be Opened
  expect _ =
    close.do(
      ub,
      f.constants,
      assets.from_lovelace(lovelace),
      f.closed(0, [], elapse_at),
      assets.from_lovelace(lovelace),
      stage_out,
    )
}
