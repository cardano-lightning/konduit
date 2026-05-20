/// Trait for rendering a Rust value as a valid Aiken expression string.
///
/// The output is suitable for embedding in `.ak` source files, e.g. as the
/// argument to `builtin.serialise_data(...)` or a `wellsigned` call.
pub trait ToAikenLiteral {
    fn to_aiken_literal(&self) -> String;
}

/// Render a byte slice as an Aiken ByteArray literal: `#"deadbeef"`.
pub fn bytes_to_hex_lit(bytes: &[u8]) -> String {
    format!("#\"{}\"", hex::encode(bytes))
}

/// Render a list of items with `ToAikenLiteral` as an Aiken list literal.
pub fn list_lit<T: ToAikenLiteral>(items: &[T]) -> String {
    if items.is_empty() {
        "[]".to_string()
    } else {
        format!(
            "[{}]",
            items
                .iter()
                .map(|x| x.to_aiken_literal())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

pub mod compat;
mod literal;
