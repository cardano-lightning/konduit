use aiken/collection/list
use aiken/interval.{Finite}
use cardano/assets
use konduit/steps/expire.{do}
use konduit/steps/expire_vectors.{vectors}
use konduit/types as t

test test_expire_vectors() {
  list.all(
    vectors,
    fn(vector) {
      let pending = (
        vector.amount,
        vector.timeout,
        #"0000000000000000000000000000000000000000000000000000000000000000",
      )
      let key = #"1111111111111111111111111111111111111111111111111111111111111111"
      trace vector.name
      do(
        Finite(vector.lower_bound),
        ("tag", key, key, 0),
        t.Responded(vector.amount, [pending]),
        assets.from_lovelace(10_000_000),
        t.Responded(0, []),
        [#"01"],
      ) == vector.expected_return
    },
  )
}
