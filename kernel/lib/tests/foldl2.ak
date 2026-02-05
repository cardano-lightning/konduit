use aiken/collection/list.{foldl2}

test test_order() {
  let src = [1, 2, 3]
  let got0, got1 <-
    foldl2(
      src,
      [],
      0,
      fn(i, acc, tot, return) { return([i, ..acc], i + tot) },
      _,
    )
  let want0 = [3, 2, 1]
  let want1 = 6
  trace (got0, got1)
  trace (want0, want1)
  (got0, got1) == (want0, want1)
}

type Fold<a, result> =
  fn(a) -> result

pub fn foldr(
  self: List<elem>,
  zero_a: a,
  with: fn(elem, a, Fold<a, result>) -> result,
  return: Fold<a, result>,
) -> result {
  do_foldr(self, with, return)(zero_a)
}

fn do_foldr(
  self: List<elem>,
  with: fn(elem, a, Fold<a, result>) -> result,
  return: Fold<a, result>,
) -> Fold<a, result> {
  when self is {
    [] -> return
    [x, ..xs] -> do_foldr(xs, with, fn(a) { with(x, a, return) })
  }
}

pub fn foldl(
  self: List<elem>,
  zero_a: a,
  with: fn(elem, a, Fold<a, result>) -> result,
  return: Fold<a, result>,
) -> result {
  do_foldl(self, with, return)(zero_a)
}

fn do_foldl(
  self: List<elem>,
  with: fn(elem, a, Fold<a, result>) -> result,
  return: Fold<a, result>,
) -> Fold<a, result> {
  when self is {
    [] -> return
    [x, ..xs] ->
      with(x, _, do_foldl(xs, with, return))
  }
}

test test_foldr_cps() {
  let src = [1, 2, 3]
  let got <-
    foldr(src, [], fn(i, acc, return) { return([i, ..acc]) }, _)
  let want = [3, 2, 1]
  got == want
}

test test_foldl_cps() {
  let src = [1, 2, 3]
  let got <-
    foldl(src, [], fn(i, acc, return) { return([i, ..acc]) }, _)
  let want = [3, 2, 1]
  got == want
}
