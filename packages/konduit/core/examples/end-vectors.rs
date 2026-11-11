fn main() {
    println!(
        "pub type EndVector {{\n  EndVector {{\n    name: ByteArray,\n    timeout: Int,\n    lower_bound: Int,\n    expected_return: ByteArray,\n  }}\n}}\n\npub const vectors: List<EndVector> = [\n  EndVector {{ name: \"end_at_timeout_boundary\", timeout: 1000, lower_bound: 1000, expected_return: #\"1111111111111111111111111111111111111111111111111111111111111111\" }},\n  EndVector {{ name: \"end_at_second_timeout_boundary\", timeout: 1001, lower_bound: 1001, expected_return: #\"1111111111111111111111111111111111111111111111111111111111111111\" }},\n]"
    )
}
