use aiken/collection/list
use aiken/interval.{Finite}
use konduit/steps/elapse.{do}
use konduit/steps/elapse_vectors.{vectors}
use konduit/types as t

test test_elapse_vectors() {
  list.all(
    vectors,
    fn(vector) {
      let key = #"1111111111111111111111111111111111111111111111111111111111111111"
      trace vector.name
      do(
        Finite(vector.lower_bound),
        ("tag", key, key, 0),
        t.Closed(0, [], vector.elapse_at),
      ) == vector.expected_return
    },
  )
}
