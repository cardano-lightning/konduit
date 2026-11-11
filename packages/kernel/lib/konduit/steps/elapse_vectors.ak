pub type ElapseVector {
  ElapseVector {
    name: ByteArray,
    lower_bound: Int,
    elapse_at: Int,
    expected_return: ByteArray,
  }
}

pub const vectors: List<ElapseVector> =
  [
    ElapseVector {
      name: "elapse_at_boundary",
      lower_bound: 1000,
      elapse_at: 1000,
      expected_return: #"1111111111111111111111111111111111111111111111111111111111111111",
    },
    ElapseVector {
      name: "elapse_at_later_boundary",
      lower_bound: 1001,
      elapse_at: 1001,
      expected_return: #"1111111111111111111111111111111111111111111111111111111111111111",
    },
  ]
