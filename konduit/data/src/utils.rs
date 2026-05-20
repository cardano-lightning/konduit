use anyhow::anyhow;

/// Converts a slice into a fixed-size array, returning an error if the length doesn't match.
pub fn try_into_array<T: Copy, const N: usize>(v: &[T]) -> anyhow::Result<[T; N]> {
    <[T; N]>::try_from(v).map_err(|_| anyhow!("Expected a slice of length {}", N))
}

/// Encodes a `Signature` as a `PlutusData` bytes value.
#[cfg(feature = "proptest")]
pub fn signature_to_plutus_data(
    signature: cardano_sdk::Signature,
) -> cardano_sdk::PlutusData<'static> {
    cardano_sdk::PlutusData::from(signature.as_ref())
}

/// Decodes a `Signature` from a `PlutusData` bytes value.
#[cfg(feature = "proptest")]
pub fn signature_from_plutus_data(
    plutus_data: &cardano_sdk::PlutusData,
) -> anyhow::Result<cardano_sdk::Signature> {
    Ok(cardano_sdk::Signature::from(*<&[u8; 64]>::try_from(
        plutus_data,
    )?))
}
