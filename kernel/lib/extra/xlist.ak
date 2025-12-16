use aiken/builtin.{head_list, tail_list}
use aiken/primitive/bytearray
use aiken/primitive/int

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

/// Like drop while, but keep first failed case
pub fn drop_until(l: List<elem>, pred: fn(elem) -> Bool) {
  when l is {
    [] -> []
    [fst, ..rest] ->
      if pred(fst) {
        drop_until(rest, pred)
      } else {
        l
      }
  }
}

/// Special case of drop until where pred(x) : x < threshold
pub fn drop_until_int(l: List<Int>, threshold: Int) {
  when l is {
    [fst, ..rest] ->
      if fst < threshold {
        drop_until_int(rest, threshold)
      } else {
        l
      }
    [] -> []
  }
}

/// Ordered insert
/// On a monotonically strictly increasing list, 
/// it will not insert item if already present,
/// else will insert into the correct place
pub fn ordered_insert(xs: List<ByteArray>, item: ByteArray) {
  when xs is {
    [x, ..xs] ->
      when bytearray.compare(x, item) is {
        Less -> [x, ..ordered_insert(xs, item)]
        Equal -> [x, ..xs]
        Greater -> fail @"Impossible"
      }
    _ -> [item]
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

pub type Fold4<a, b, c, d, result> =
  fn(a, b, c, d) -> result

pub fn foldl4(
  self: List<elem>,
  zero_a: a,
  zero_b: b,
  zero_c: c,
  zero_d: d,
  with: fn(elem, a, b, c, d, Fold4<a, b, c, d, result>) -> result,
  return: Fold4<a, b, c, d, result>,
) -> result {
  do_foldl4(self, with, return)(zero_a, zero_b, zero_c, zero_d)
}

fn do_foldl4(
  self: List<elem>,
  with: fn(elem, a, b, c, d, Fold4<a, b, c, d, result>) -> result,
  return: Fold4<a, b, c, d, result>,
) -> Fold4<a, b, c, d, result> {
  when self is {
    [] -> return
    [x, ..xs] ->
      do_foldl4(xs, with, fn(a, b, c, d) { with(x, a, b, c, d, return) })
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

pub type Fold6<a, b, c, d, e, f, result> =
  fn(a, b, c, d, e, f) -> result

pub fn foldl6(
  self: List<elem>,
  zero_a: a,
  zero_b: b,
  zero_c: c,
  zero_d: d,
  zero_e: e,
  zero_f: f,
  with: fn(elem, a, b, c, d, e, f, Fold6<a, b, c, d, e, f, result>) -> result,
  return: Fold6<a, b, c, d, e, f, result>,
) -> result {
  do_foldl6(self, with, return)(zero_a, zero_b, zero_c, zero_d, zero_e, zero_f)
}

fn do_foldl6(
  self: List<elem>,
  with: fn(elem, a, b, c, d, e, f, Fold6<a, b, c, d, e, f, result>) -> result,
  return: Fold6<a, b, c, d, e, f, result>,
) -> Fold6<a, b, c, d, e, f, result> {
  when self is {
    [] -> return
    [x, ..xs] ->
      do_foldl6(
        xs,
        with,
        fn(a, b, c, d, e, f) { with(x, a, b, c, d, e, f, return) },
      )
  }
}

pub type Fold7<a, b, c, d, e, f, g, result> =
  fn(a, b, c, d, e, f, g) -> result

pub fn foldl7(
  self: List<elem>,
  zero_a: a,
  zero_b: b,
  zero_c: c,
  zero_d: d,
  zero_e: e,
  zero_f: f,
  zero_g: g,
  with: fn(elem, a, b, c, d, e, f, g, Fold7<a, b, c, d, e, f, g, result>) ->
    result,
  return: Fold7<a, b, c, d, e, f, g, result>,
) -> result {
  do_foldl7(self, with, return)(
    zero_a,
    zero_b,
    zero_c,
    zero_d,
    zero_e,
    zero_f,
    zero_g,
  )
}

fn do_foldl7(
  self: List<elem>,
  with: fn(elem, a, b, c, d, e, f, g, Fold7<a, b, c, d, e, f, g, result>) ->
    result,
  return: Fold7<a, b, c, d, e, f, g, result>,
) -> Fold7<a, b, c, d, e, f, g, result> {
  when self is {
    [] -> return
    [x, ..xs] ->
      do_foldl7(
        xs,
        with,
        fn(a, b, c, d, e, f, g) { with(x, a, b, c, d, e, f, g, return) },
      )
  }
}

pub type Fold8<a, b, c, d, e, f, g, h, result> =
  fn(a, b, c, d, e, f, g, h) -> result

pub fn foldl8(
  self: List<elem>,
  zero_a: a,
  zero_b: b,
  zero_c: c,
  zero_d: d,
  zero_e: e,
  zero_f: f,
  zero_g: g,
  zero_h: h,
  with: fn(elem, a, b, c, d, e, f, g, h, Fold8<a, b, c, d, e, f, g, h, result>) ->
    result,
  return: Fold8<a, b, c, d, e, f, g, h, result>,
) -> result {
  do_foldl8(self, with, return)(
    zero_a,
    zero_b,
    zero_c,
    zero_d,
    zero_e,
    zero_f,
    zero_g,
    zero_h,
  )
}

fn do_foldl8(
  self: List<elem>,
  with: fn(elem, a, b, c, d, e, f, g, h, Fold8<a, b, c, d, e, f, g, h, result>) ->
    result,
  return: Fold8<a, b, c, d, e, f, g, h, result>,
) -> Fold8<a, b, c, d, e, f, g, h, result> {
  when self is {
    [] -> return
    [x, ..xs] ->
      do_foldl8(
        xs,
        with,
        fn(a, b, c, d, e, f, g, h) { with(x, a, b, c, d, e, f, g, h, return) },
      )
  }
}

pub type Fold9<a, b, c, d, e, f, g, h, i, result> =
  fn(a, b, c, d, e, f, g, h, i) -> result

pub fn foldl9(
  self: List<elem>,
  zero_a: a,
  zero_b: b,
  zero_c: c,
  zero_d: d,
  zero_e: e,
  zero_f: f,
  zero_g: g,
  zero_h: h,
  zero_i: i,
  with: fn(
    elem,
    a,
    b,
    c,
    d,
    e,
    f,
    g,
    h,
    i,
    Fold9<a, b, c, d, e, f, g, h, i, result>,
  ) ->
    result,
  return: Fold9<a, b, c, d, e, f, g, h, i, result>,
) -> result {
  do_foldl9(self, with, return)(
    zero_a,
    zero_b,
    zero_c,
    zero_d,
    zero_e,
    zero_f,
    zero_g,
    zero_h,
    zero_i,
  )
}

fn do_foldl9(
  self: List<elem>,
  with: fn(
    elem,
    a,
    b,
    c,
    d,
    e,
    f,
    g,
    h,
    i,
    Fold9<a, b, c, d, e, f, g, h, i, result>,
  ) ->
    result,
  return: Fold9<a, b, c, d, e, f, g, h, i, result>,
) -> Fold9<a, b, c, d, e, f, g, h, i, result> {
  when self is {
    [] -> return
    [x, ..xs] ->
      do_foldl9(
        xs,
        with,
        fn(a, b, c, d, e, f, g, h, i) {
          with(x, a, b, c, d, e, f, g, h, i, return)
        },
      )
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

pub fn drop2_with(
  xs: List<a>,
  ys: List<a>,
  n: Int,
  with: fn(a, a) -> Bool,
  return: fn(List<a>, List<a>) -> result,
) -> result {
  if n == 0 {
    return(xs, ys)
  } else {
    expect with(head_list(xs), head_list(ys))
    drop2_with(tail_list(xs), tail_list(ys), n - 1, with, return)
  }
}

test test_drop2_with() {
  let l = [1, 2, 3, 4, 5]
  let r = [1, 2, 5]
  let with =
    fn(x, y) { x == y }
  let l, _r <-
    drop2_with(l, r, 2, with, _)
  l == [3, 4, 5]
}
