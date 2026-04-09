use aiken/interval.{Finite}
use konduit/steps/end
use konduit/tests/fixture as f

// end.do: Responded -> consumed (EOL, no output)
// Requires all pending timeouts <= lower_bound

const expired_pend = (1_000_000, f.now - 1, f.secret)

test test_end_empty_pends() {
  let lb = Finite(f.now)
  let vkey = end.do(lb, f.constants, f.responded(0, []))
  vkey == f.add_vkey
}

test test_end_all_expired() {
  let lb = Finite(f.now)
  let pends = [expired_pend, expired_pend]
  let vkey = end.do(lb, f.constants, f.responded(2_000_000, pends))
  vkey == f.add_vkey
}

test test_end_exact_timeout() {
  // timeout == lower_bound is valid (<=)
  let lb = Finite(f.now)
  let pend = (1_000_000, f.now, f.secret)
  let vkey = end.do(lb, f.constants, f.responded(1_000_000, [pend]))
  vkey == f.add_vkey
}

test test_end_pending_not_expired() fail {
  // a pending whose timeout > lower_bound
  let lb = Finite(f.now)
  let live_pend = (1_000_000, f.now + 1, f.secret)
  expect _ = end.do(lb, f.constants, f.responded(1_000_000, [live_pend]))
}

test test_end_mixed_one_live() fail {
  let lb = Finite(f.now)
  let pends = [expired_pend, (500_000, f.now + 1, f.secret)]
  expect _ = end.do(lb, f.constants, f.responded(1_500_000, pends))
}

test test_end_wrong_stage() fail {
  let lb = Finite(f.now)
  expect _ = end.do(lb, f.constants, f.opened(0, []))
}
