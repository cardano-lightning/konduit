fn main() {
    println!(
        "pub type UnlockVector {{\n  UnlockVector {{\n    name: ByteArray,\n    upper_bound: Int,\n    amount: Int,\n    timeout: Int,\n    expected_return: ByteArray,\n  }}\n}}\n\npub const vectors: List<UnlockVector> = [\n  UnlockVector {{ name: \"unlock_matching_preimage_at_timeout\", upper_bound: 1000, amount: 10, timeout: 1000, expected_return: #\"1111111111111111111111111111111111111111111111111111111111111111\" }},\n]"
    )
}
