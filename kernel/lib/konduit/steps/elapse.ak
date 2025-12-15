use aiken/crypto.{VerificationKey}
use aiken/interval.{Finite}
use konduit/prelude.{Timebound}
use konduit/types as t

pub fn do(
  lower_bound: Timebound,
  constants: t.Constants,
  stage_in: t.Stage,
) -> VerificationKey {
  let t.Constants { add_vkey, .. } = constants
  expect t.Closed(_, _, elapse_at) = stage_in
  expect Finite(lower_bound) = lower_bound
  expect lower_bound <= elapse_at
  add_vkey
}
