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
  let t.Constants { add_vkey, close_period, .. } = constants
  expect t.Opened(subbed) = stage_in
  expect t.Closed(subbed_, elapse_at) = stage_out
  expect subbed == subbed_
  expect amount_in(value_in) <= amount_out(value_out)
  return(add_vkey, None, Some(elapse_at - close_period))
}
