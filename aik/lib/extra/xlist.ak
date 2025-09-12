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

pub type Fold5<a, b, c, d, e, result> =
  fn(a, b, c, d, e) -> result

pub fn foldl5(
  self: List<elem>,
  zero_a: a,
  zero_b: b,
  zero_c: c,
  zero_d: d,
  zero_e: e,
  with: fn(elem, a, b, c, d, e, Fold5<a, b, c, d, e, result>) -> result,
  return: Fold5<a, b, c, d, e, result>,
) -> result {
  do_foldl5(self, with, return)(zero_a, zero_b, zero_c, zero_d, zero_e)
}

fn do_foldl5(
  self: List<elem>,
  with: fn(elem, a, b, c, d, e, Fold5<a, b, c, d, e, result>) -> result,
  return: Fold5<a, b, c, d, e, result>,
) -> Fold5<a, b, c, d, e, result> {
  when self is {
    [] -> return
    [x, ..xs] ->
      do_foldl5(xs, with, fn(a, b, c, d, e) { with(x, a, b, c, d, e, return) })
  }
}

pub type YeildWith<a, result> =
  fn(a, List<a>) -> result

fn yield_with(
  self: List<a>,
  with: fn(a) -> Bool,
  return: YeildWith<a, result>,
) -> result {
  when self is {
    [a, ..self] ->
      if with(a) {
        return(a, self)
      } else {
        yield_with(self, with, return)
      }
    _ -> fail @"impossible"
  }
}
