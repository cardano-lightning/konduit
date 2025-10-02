use cardano/assets.{Value}
use konduit/assets.{amount_in, amount_out} as k_assets
use konduit/steps/common.{Return}
use konduit/types as t

pub fn do(
  constants: t.Constants,
  value_in: Value,
  stage_in: t.Stage,
  value_out: Value,
  stage_out: t.Stage,
  return: Return<result>,
) -> result {
  let t.Constants { add_vkey, .. } = constants
  expect t.Opened(_) = stage_in
  expect stage_in == stage_out
  expect amount_in(value_in) < amount_out(value_out)
  return(add_vkey, None, None)
}
