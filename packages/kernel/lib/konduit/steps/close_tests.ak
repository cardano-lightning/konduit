use aiken/collection/list
use aiken/interval.{Finite}
use cardano/assets
use konduit/steps/close.{do}
use konduit/steps/close_vectors.{vectors}
use konduit/types as t

test test_close_vectors() {
  list.all(
    vectors,
    fn(vector) {
      let key = #"1111111111111111111111111111111111111111111111111111111111111111"
      trace vector.name
      do(
        Finite(vector.upper_bound),
        ("tag", key, key, vector.close_period),
        assets.from_lovelace(10),
        t.Opened(5, [(1, 2)]),
        assets.from_lovelace(10),
        t.Closed(5, [(1, 2)], vector.elapse_at),
      ) == vector.expected_return
    },
  )
}
