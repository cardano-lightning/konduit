fn main() {
    println!(
        "pub type ElapseVector {{\n  ElapseVector {{\n    name: ByteArray,\n    lower_bound: Int,\n    elapse_at: Int,\n    expected_return: ByteArray,\n  }}\n}}\n\npub const vectors: List<ElapseVector> = [\n  ElapseVector {{ name: \"elapse_at_boundary\", lower_bound: 1000, elapse_at: 1000, expected_return: #\"1111111111111111111111111111111111111111111111111111111111111111\" }},\n  ElapseVector {{ name: \"elapse_at_later_boundary\", lower_bound: 1001, elapse_at: 1001, expected_return: #\"1111111111111111111111111111111111111111111111111111111111111111\" }},\n]"
    )
}
