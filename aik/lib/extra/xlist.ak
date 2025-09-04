/// Drop element or fail 
pub fn drop_hard(l: List<elem>, i: elem) {
  when l is {
    [] -> fail @"element must be in list"
    [fst, ..l] ->
      if fst == i {
        l
      } else {
        drop_hard(l, i)
      }
  }
}

pub type Fold3<a, b, c, result> =
  fn(a, b, c) -> result

pub fn foldl3(
  self: List<elem>,
  zero_a: a,
  zero_b: b,
  zero_c: c,
  with: fn(elem, a, b, c, Fold3<a, b, c, result>) -> result,
  return: Fold3<a, b, c, result>,
) -> result {
  do_foldl3(self, with, return)(zero_a, zero_b, zero_c)
}

fn do_foldl3(
  self: List<elem>,
  with: fn(elem, a, b, c, Fold3<a, b, c, result>) -> result,
  return: Fold3<a, b, c, result>,
) -> Fold3<a, b, c, result> {
  when self is {
    [] -> return
    [x, ..xs] -> do_foldl3(xs, with, fn(a, b, c) { with(x, a, b, c, return) })
  }
}
