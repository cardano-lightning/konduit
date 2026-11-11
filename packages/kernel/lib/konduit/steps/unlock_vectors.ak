pub type UnlockVector {
  UnlockVector {
    name: ByteArray,
    upper_bound: Int,
    amount: Int,
    timeout: Int,
    expected_return: ByteArray,
  }
}

pub const vectors: List<UnlockVector> =
  [
    UnlockVector {
      name: "unlock_matching_preimage_at_timeout",
      upper_bound: 1000,
      amount: 10,
      timeout: 1000,
      expected_return: #"1111111111111111111111111111111111111111111111111111111111111111",
    },
  ]
