use aiken/builtin.{sha2_256}
use aiken/collection/list
use aiken/interval.{Finite}
use cardano/assets
use konduit/steps/unlock.{do}
use konduit/steps/unlock_vectors.{vectors}
use konduit/types as t

test test_unlock_vectors() {
  list.all(
    vectors,
    fn(vector) {
      let secret = #"0000000000000000000000000000000000000000000000000000000000000000"
      let pending = (vector.amount, vector.timeout, sha2_256(secret))
      let key = #"1111111111111111111111111111111111111111111111111111111111111111"
      trace vector.name
      do(
        Finite(vector.upper_bound),
        ("tag", key, key, 0),
        assets.from_lovelace(vector.amount + 1),
        t.Responded(vector.amount, [pending]),
        assets.from_lovelace(1),
        t.Responded(0, []),
        [secret],
      ) == vector.expected_return
    },
  )
}
