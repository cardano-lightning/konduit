use aiken/interval.{Finite}
use cardano/assets
use konduit/steps/expire
use konduit/tests/fixture as f

// expire.do / expire.account: Responded -> Responded
// Removes timed-out pendings (lower_bound > timeout).
// Unpend #"" = continue (keep pend), anything else = expire that pend.

const amount = 1_000_000

// A pending whose timeout has passed
const expired_pend = (amount, f.now - 1, f.secret)

// A pending whose timeout has not passed
const live_pend = (amount, f.now + 1, f.secret)

const lb = Finite(f.now)

// ── expire.account ────────────────────────────────────────────────────────────

test test_expire_account_empty() {
  let total = expire.account(lb, [], [], [])
  total == 0
}

test test_expire_account_expire_one() {
  // #"aa" is a non-empty unpend -> expire
  let total = expire.account(lb, [#"aa"], [expired_pend], [])
  total == amount
}

test test_expire_account_continue_one() {
  // #"" -> keep pend in both in and out
  let total = expire.account(lb, [#""], [live_pend], [live_pend])
  total == 0
}

test test_expire_account_mixed() {
  // expire first, continue second
  let unpends = [#"aa", #""]
  let pends_in = [expired_pend, live_pend]
  let pends_out = [live_pend]
  let total = expire.account(lb, unpends, pends_in, pends_out)
  total == amount
}

test test_expire_account_expire_not_timed_out() fail {
  // lower_bound <= timeout means NOT expired; can't expire it
  expect _ = expire.account(lb, [#"aa"], [live_pend], [])
}

test test_expire_account_continue_mismatch() fail {
  // continue: pend must appear in both in and out
  expect _ = expire.account(lb, [#""], [live_pend], [expired_pend])
}

// ── expire.do ─────────────────────────────────────────────────────────────────

test test_expire_do_ok() {
  // value_out must cover pends_amount_out + min_ada_buffer (2 ADA)
  // After expiry pends_amount_out = 0, so value_out just needs >= 2 ADA
  let stage_in = f.responded(amount, [expired_pend])
  let stage_out = f.responded(0, [])
  let vkey =
    expire.do(
      lb,
      f.constants,
      stage_in,
      assets.from_lovelace(2_000_000),
      stage_out,
      [#"aa"],
    )
  vkey == f.add_vkey
}

test test_expire_do_wrong_stage() fail {
  expect _ =
    expire.do(
      lb,
      f.constants,
      f.opened(0, []),
      assets.from_lovelace(amount),
      f.responded(0, []),
      [],
    )
}
