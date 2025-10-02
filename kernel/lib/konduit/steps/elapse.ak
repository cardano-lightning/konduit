use konduit/steps/common.{Return}
use konduit/types as t

pub fn do(
  constants: t.Constants,
  stage_in: t.Stage,
  return: Return<result>,
) -> result {
  let t.Constants { add_vkey, .. } = constants
  expect t.Closed(_, expire_at) = stage_in
  return(add_vkey, Some(expire_at), None)
}
