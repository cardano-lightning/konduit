use crate::ParseError;

/// Converts a slice into a fixed-size array, returning an error if the length doesn't match.
pub fn try_into_array<T: Copy, const N: usize>(v: &[T]) -> Result<[T; N], ParseError> {
    <[T; N]>::try_from(v).map_err(|_| ParseError::WrongLength {
        expected: N,
        got: v.len(),
    })
}
