pub type EndVector {
  EndVector {
    name: ByteArray,
    timeout: Int,
    lower_bound: Int,
    expected_return: ByteArray,
  }
}

pub const vectors: List<EndVector> =
  [
    EndVector {
      name: "end_at_timeout_boundary",
      timeout: 1000,
      lower_bound: 1000,
      expected_return: #"1111111111111111111111111111111111111111111111111111111111111111",
    },
    EndVector {
      name: "end_at_second_timeout_boundary",
      timeout: 1001,
      lower_bound: 1001,
      expected_return: #"1111111111111111111111111111111111111111111111111111111111111111",
    },
  ]
