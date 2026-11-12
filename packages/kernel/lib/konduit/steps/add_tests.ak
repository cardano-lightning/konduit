use aiken/collection/list
use cardano/assets
use konduit/steps/add.{do}
use konduit/steps/add_vectors.{vectors}
use konduit/types as t

test test_add_vectors() {
  list.all(
    vectors,
    fn(vector) {
      let key = #"1111111111111111111111111111111111111111111111111111111111111111"
      let stage = t.Opened(0, [])
      trace vector.name
      do(
        ("tag", key, key, 0),
        assets.from_lovelace(vector.input),
        stage,
        assets.from_lovelace(vector.output),
        stage,
      ) == vector.expected_return
    },
  )
}
