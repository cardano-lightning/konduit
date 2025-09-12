use aiken/collection/dict.{to_pairs}
use cardano/assets.{Value, to_dict}

/// Assumes value is from utxo so must contain ada
pub fn only_ada(v: Value) -> Int {
  expect [Pair(_, d)] = v |> to_dict |> to_pairs
  expect [Pair(_, amount)] = d |> to_pairs
  amount
}
