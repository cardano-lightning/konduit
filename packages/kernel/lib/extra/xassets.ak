use cardano/value.{to_pairs as value_to_pairs}

/// Assumes value is from utxo so must contain ada
pub fn only_ada(v: Value) -> Int {
  expect [Pair(_, d)] = v |> value_to_pairs
  expect [Pair(_, amount)] = d
  amount
}
