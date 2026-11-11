fn main() {
    println!(
        "pub type AddVector {{\n  AddVector {{\n    name: ByteArray,\n    input: Int,\n    output: Int,\n    expected_return: ByteArray,\n  }}\n}}\n\npub const vectors: List<AddVector> = [\n  AddVector {{ name: \"add_increases_value\", input: 100, output: 101, expected_return: #\"1111111111111111111111111111111111111111111111111111111111111111\" }},\n  AddVector {{ name: \"add_larger_increase\", input: 100, output: 150, expected_return: #\"1111111111111111111111111111111111111111111111111111111111111111\" }},\n]"
    )
}
