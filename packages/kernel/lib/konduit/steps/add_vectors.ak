pub type AddVector {
  AddVector {
    name: ByteArray,
    input: Int,
    output: Int,
    expected_return: ByteArray,
  }
}

pub const vectors: List<AddVector> =
  [
    AddVector {
      name: "add_increases_value",
      input: 100,
      output: 101,
      expected_return: #"1111111111111111111111111111111111111111111111111111111111111111",
    },
    AddVector {
      name: "add_larger_increase",
      input: 100,
      output: 150,
      expected_return: #"1111111111111111111111111111111111111111111111111111111111111111",
    },
  ]
