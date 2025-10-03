use aiken/collection/list 
use extra/xoption
use konduit/steps/common.{Return}
use konduit/types as t

pub fn do(
  constants: t.Constants,
  stage_in: t.Stage,
  return: Return<result>,
) -> result {
  let t.Constants { add_vkey, .. } = constants
  expect t.Responded(_, pends_in) = stage_in
  let lower_bound =
    pends_in
      |> list.foldl(None, fn((_, timeout, _), c) { xoption.max(c, timeout) })
  return(add_vkey, lower_bound, None)
}
