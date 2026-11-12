use aiken/collection/list
use cardano/assets
use konduit/steps/sub.{do}
use konduit/steps/sub_vectors.{SubVector, vectors}
use konduit/types as t

fn vector_named(name: ByteArray) -> SubVector {
  expect Some(vector) = list.find(vectors, fn(vector) { vector.name == name })
  vector
}

fn evaluate(vector: SubVector, minimum: Int) -> ByteArray {
  let key = vector.key
  do(
    vector.upper_bound,
    (vector.tag, key, key, 0),
    assets.from_lovelace(minimum + 1),
    t.Opened(0, vector.useds_in),
    assets.from_lovelace(1),
    t.Opened(minimum, vector.useds_out),
    vector.squash,
    vector.unlockeds,
  )
}

test test_sub_success_vectors() {
  list.all(
    vectors,
    fn(vector) {
      if vector.expect_failure {
        True
      } else {
        trace vector.name
        evaluate(vector, vector.expected) == vector.expected_return
      }
    },
  )
}

test test_unlocked_timeout_too_early() fail {
  let vector = vector_named("test_unlocked_timeout_too_early")
  vector.expect_failure && vector.failure_reason != "" && evaluate(
    vector,
    vector.expected,
  ) == vector.expected_return
}

test test_invalid_squash_signature() fail {
  let vector = vector_named("test_invalid_squash_signature")
  vector.expect_failure && vector.failure_reason != "" && evaluate(
    vector,
    vector.expected,
  ) == vector.expected_return
}

test test_invalid_unlocked_signature() fail {
  let vector = vector_named("test_invalid_unlocked_signature")
  vector.expect_failure && vector.failure_reason != "" && evaluate(
    vector,
    vector.expected,
  ) == vector.expected_return
}

test test_unlocked_mismatches_output() fail {
  let vector = vector_named("test_unlocked_mismatches_output")
  vector.expect_failure && vector.failure_reason != "" && evaluate(
    vector,
    vector.expected,
  ) == vector.expected_return
}

test test_ooo_useds_outs() fail {
  let vector = vector_named("test_ooo_useds_outs")
  vector.expect_failure && vector.failure_reason != "" && evaluate(
    vector,
    vector.expected,
  ) == vector.expected_return
}

test test_ooo_useds_outs_1() fail {
  let vector = vector_named("test_ooo_useds_outs_1")
  vector.expect_failure && vector.failure_reason != "" && evaluate(
    vector,
    vector.expected,
  ) == vector.expected_return
}

test test_useds_out_too_long() fail {
  let vector = vector_named("test_useds_out_too_long")
  vector.expect_failure && vector.failure_reason != "" && evaluate(
    vector,
    vector.expected,
  ) == vector.expected_return
}

test test_used_already_squashed() fail {
  let vector = vector_named("test_used_already_squashed")
  vector.expect_failure && vector.failure_reason != "" && evaluate(
    vector,
    vector.expected,
  ) == vector.expected_return
}

test test_account_null_rejects_one_more() fail {
  let vector = vector_named("test_account_null")
  evaluate(vector, vector.expected + 1) == vector.expected_return
}

test test_account_scenario_0_rejects_one_more() fail {
  let vector = vector_named("test_account_scenario_0")
  evaluate(vector, vector.expected + 1) == vector.expected_return
}

test test_account_scenario_1_rejects_one_more() fail {
  let vector = vector_named("test_account_scenario_1")
  evaluate(vector, vector.expected + 1) == vector.expected_return
}

test test_with_squash_rejects_one_more() fail {
  let vector = vector_named("test_with_squash")
  evaluate(vector, vector.expected + 1) == vector.expected_return
}

test test_excluded_index_is_skipped_rejects_one_more() fail {
  let vector = vector_named("test_excluded_index_is_skipped")
  evaluate(vector, vector.expected + 1) == vector.expected_return
}

test test_used_not_squashed_rejects_one_more() fail {
  let vector = vector_named("test_used_not_squashed")
  evaluate(vector, vector.expected + 1) == vector.expected_return
}

test test_unlocked_already_used_rejects_one_more() fail {
  let vector = vector_named("test_unlocked_already_used")
  evaluate(vector, vector.expected + 1) == vector.expected_return
}

test test_big_scenario_rejects_one_more() fail {
  let vector = vector_named("test_big_scenario")
  evaluate(vector, vector.expected + 1) == vector.expected_return
}
