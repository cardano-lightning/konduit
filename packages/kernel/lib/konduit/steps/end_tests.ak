use aiken/collection/list
use aiken/interval.{Finite}
use konduit/steps/end.{do}
use konduit/steps/end_vectors.{vectors}
use konduit/types as t

test test_end_vectors() {
  list.all(
    vectors,
    fn(vector) {
      let key = #"1111111111111111111111111111111111111111111111111111111111111111"
      trace vector.name
      do(
        Finite(vector.lower_bound),
        ("tag", key, key, 0),
        t.Responded(0, [(1, vector.timeout, #"")]),
      ) == vector.expected_return
    },
  )
}
