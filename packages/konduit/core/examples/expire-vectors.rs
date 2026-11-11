fn main() {
    println!(
        "pub type ExpireVector {{\n  ExpireVector {{\n    name: ByteArray,\n    lower_bound: Int,\n    amount: Int,\n    timeout: Int,\n    expected_return: ByteArray,\n  }}\n}}\n\npub const vectors: List<ExpireVector> = [\n  ExpireVector {{ name: \"expire_at_timeout_boundary\", lower_bound: 1000, amount: 10, timeout: 1000, expected_return: #\"1111111111111111111111111111111111111111111111111111111111111111\" }},\n]"
    )
}
