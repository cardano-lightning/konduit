fn main() {
    println!(
        "pub type CloseVector {{\n  CloseVector {{\n    name: ByteArray,\n    upper_bound: Int,\n    elapse_at: Int,\n    close_period: Int,\n    expected_return: ByteArray,\n  }}\n}}\n\npub const vectors: List<CloseVector> = [\n  CloseVector {{ name: \"close_at_period_boundary\", upper_bound: 1000, elapse_at: 1100, close_period: 100, expected_return: #\"1111111111111111111111111111111111111111111111111111111111111111\" }},\n  CloseVector {{ name: \"close_second_period_boundary\", upper_bound: 1000, elapse_at: 1200, close_period: 200, expected_return: #\"1111111111111111111111111111111111111111111111111111111111111111\" }},\n]"
    )
}
