use anyhow::anyhow;

pub fn try_into_array<T, const N: usize>(v: Vec<T>) -> anyhow::Result<[T; N]> {
    <[T; N]>::try_from(v)
        .map_err(|v: Vec<T>| anyhow!("Expected a Vec of length {}, but got {}", N, v.len()))
}
