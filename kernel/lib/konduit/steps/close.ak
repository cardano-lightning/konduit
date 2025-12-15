use aiken/crypto.{VerificationKey}
use aiken/interval.{Finite}
use cardano/assets.{Value}
use konduit/assets.{amount_in, amount_out} as k_assets
use konduit/prelude.{Timebound}
use konduit/types as t

pub fn do(
  upper_bound: Timebound,
  constants: t.Constants,
  value_in: Value,
  stage_in: t.Stage,
  value_out: Value,
  stage_out: t.Stage,
) -> VerificationKey {
  let t.Constants { add_vkey, close_period, .. } = constants
  expect t.Opened(subbed, useds) = stage_in
  expect t.Closed(subbed_, useds_, elapse_at) = stage_out
  expect subbed == subbed_
  expect useds == useds_
  expect amount_in(value_in) <= amount_out(value_out)
  expect Finite(upper_bound) = upper_bound
  expect upper_bound <= elapse_at - close_period
  add_vkey
}
