use aiken/crypto.{VerificationKey}
use cardano/assets.{Value}
use konduit/assets.{amount_in, amount_out} as k_assets
use konduit/types as t

pub fn do(
  constants: t.Constants,
  value_in: Value,
  stage_in: t.Stage,
  value_out: Value,
  stage_out: t.Stage,
) -> VerificationKey {
  let (_tag, add_vkey, _sub_vkey, _close_period) = constants
  expect t.Opened(_, _) = stage_in
  expect stage_in == stage_out
  expect amount_in(value_in) < amount_out(value_out)
  add_vkey
}
