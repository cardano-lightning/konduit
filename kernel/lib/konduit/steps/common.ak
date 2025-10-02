use aiken/crypto.{VerificationKey}
use extra/xlist.{drop_hard}
use konduit/prelude.{Index}
use konduit/types as t

pub type Return<result> =
  fn(VerificationKey, Option<Int>, Option<Int>) -> result

pub fn next_index(
  prev: Index,
  exclude: t.Exclude,
  curr: Index,
  return: fn(Index, t.Exclude) -> result,
) -> result {
  if prev < curr {
    return(curr, [])
  } else {
    return(prev, drop_hard(exclude, curr))
  }
}
