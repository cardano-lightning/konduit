use aiken/crypto.{VerificationKey}
use aiken/interval.{Finite}
use konduit/prelude.{Timebound}
use konduit/types as t

pub fn do(
  lower_bound: Timebound,
  constants: t.Constants,
  stage_in: t.Stage,
) -> VerificationKey {
  let (_tag, add_vkey, _sub_vkey, _close_period) = constants
  expect t.Closed(_, _, elapse_at) = stage_in
  expect Finite(lower_bound) = lower_bound
  expect elapse_at <= lower_bound
  add_vkey
}
