pub type CloseVector {
  CloseVector {
    name: ByteArray,
    upper_bound: Int,
    elapse_at: Int,
    close_period: Int,
    expected_return: ByteArray,
  }
}

pub const vectors: List<CloseVector> =
  [
    CloseVector {
      name: "close_at_period_boundary",
      upper_bound: 1000,
      elapse_at: 1100,
      close_period: 100,
      expected_return: #"1111111111111111111111111111111111111111111111111111111111111111",
    },
    CloseVector {
      name: "close_second_period_boundary",
      upper_bound: 1000,
      elapse_at: 1200,
      close_period: 200,
      expected_return: #"1111111111111111111111111111111111111111111111111111111111111111",
    },
  ]
