fn with_no_and(x: Int, y: Int) {
  expect x == y
  expect 2 * x == 2 * y
  expect 3 * x == 3 * y
}

test test_no_and() {
  let x = 1
  let y = x
  with_no_and(x, y)
}

fn with_and(x: Int, y: Int) {
  expect and {
      x == y,
      2 * x == 2 * y,
      3 * x == 3 * y,
    }
}

test test_and() {
  let x = 1
  let y = x
  with_and(x, y)
}
