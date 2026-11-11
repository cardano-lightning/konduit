use aiken/collection/list
use aiken/interval.{Finite}
use cardano/assets
use konduit/steps/respond.{do}
use konduit/steps/respond_vectors.{key, squash, tag, vectors}
use konduit/types as t

test test_respond_vectors() {
  list.all(
    vectors,
    fn(vector) {
      trace vector.name
      do(
        Finite(vector.upper_bound),
        (tag, key, key, 0),
        assets.from_lovelace(10),
        t.Closed(0, [], 0),
        assets.from_lovelace(10),
        t.Responded(0, []),
        squash,
        [],
      ) == vector.expected_return
    },
  )
}
