use aiken/builtin.{head_list, sha2_256, tail_list}
use aiken/crypto.{VerificationKey}
use aiken/interval.{Finite}
use aiken/primitive/bytearray
use cardano/assets.{Value}
use extra/xlist.{foldl3}
use konduit/assets.{amount_in, amount_out} as k_assets
use konduit/prelude.{Amount, Timebound}
use konduit/types as t

pub fn do(
  upper_bound: Timebound,
  constants: t.Constants,
  value_in: Value,
  stage_in: t.Stage,
  value_out: Value,
  stage_out: t.Stage,
  unpends: List<t.Unpend>,
) -> VerificationKey {
  let t.Constants { sub_vkey, .. } = constants
  expect t.Responded(pends_amount_in, pends_in) = stage_in
  expect t.Responded(pends_amount_out, pends_out) = stage_out
  let subbed = amount_in(value_in) - amount_out(value_out)
  let amount = account(upper_bound, unpends, pends_in, pends_out)
  expect pends_amount_out == pends_amount_in - amount
  expect amount >= subbed
  sub_vkey
}

fn account(
  upper_bound: Timebound,
  unpends: List<t.Unpend>,
  pends_in: List<t.Pending>,
  pends_out: List<t.Pending>,
) -> Amount {
  let with =
    with_(upper_bound, _, _, _, _, _)
  let amount, pends_in, pends_out <-
    foldl3(unpends, 0, pends_in, pends_out, with, _)
  expect pends_in == pends_out
  amount
}

type ReturnWith<result> =
  fn(Amount, List<t.Pending>, List<t.Pending>) -> result

fn with_(
  upper_bound: Timebound,
  unpend: t.Unpend,
  total: Amount,
  pends_in: List<t.Pending>,
  pends_out: List<t.Pending>,
  return: ReturnWith<result>,
) -> result {
  when unpend is {
    #"" -> {
      // Continue
      expect head_list(pends_in) == head_list(pends_out)
      return(total, tail_list(pends_in), tail_list(pends_out))
    }
    _ -> {
      // Unlock
      let (amount, timeout, hash) = head_list(pends_in)
      expect sha2_256(unpend) == hash
      expect bytearray.length(unpend) == 32
      expect Finite(upper_bound) = upper_bound
      expect upper_bound <= timeout
      return(total + amount, tail_list(pends_in), pends_out)
    }
  }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

const test_amount = 1_000_000

// Timestamps: soon = now + 60s, later = now + 120s
const test_now = 1_765_889_000_000

const test_soon = test_now + 60_000

const test_later = test_soon + 60_000

const test_secret =
  #"0000000000000000000000000000000000000000000000000000000000000000"

const test_ub = Finite(test_soon)

// account tests (private fn, must be inline)

test test_unlock_account_empty() {
  let total = account(test_ub, [], [], [])
  total == 0
}

test test_unlock_account_unlock_one() {
  let secret = test_secret
  let hash = sha2_256(secret)
  let pend = (test_amount, test_later, hash)
  let total = account(test_ub, [secret], [pend], [])
  total == test_amount
}

test test_unlock_account_continue_one() {
  // #"" passes the pend through unchanged
  let secret = test_secret
  let hash = sha2_256(secret)
  let pend = (test_amount, test_later, hash)
  let total = account(test_ub, [#""], [pend], [pend])
  total == 0
}

test test_unlock_account_wrong_preimage_length() fail {
  // secret must be exactly 32 bytes
  let short_secret = #"aabb"
  let hash = sha2_256(short_secret)
  let pend = (test_amount, test_later, hash)
  expect _ = account(test_ub, [short_secret], [pend], [])
}

test test_unlock_account_timed_out() fail {
  // upper_bound > timeout: the pend has expired, can't unlock
  let secret = test_secret
  let hash = sha2_256(secret)
  let expired_timeout = test_soon - 1
  let pend = (test_amount, expired_timeout, hash)
  expect _ = account(test_ub, [secret], [pend], [])
}
