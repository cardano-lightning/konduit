pub type ExpireVector {
  ExpireVector {
    name: ByteArray,
    lower_bound: Int,
    amount: Int,
    timeout: Int,
    expected_return: ByteArray,
  }
}

pub const vectors: List<ExpireVector> =
  [
    ExpireVector {
      name: "expire_at_timeout_boundary",
      lower_bound: 1000,
      amount: 10,
      timeout: 1000,
      expected_return: #"1111111111111111111111111111111111111111111111111111111111111111",
    },
  ]
