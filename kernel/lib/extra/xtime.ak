use aiken/interval.{Finite, IntervalBound}

pub fn unbound(b: IntervalBound<Int>) {
  expect IntervalBound { bound_type: Finite(b), .. } = b
  b
}
