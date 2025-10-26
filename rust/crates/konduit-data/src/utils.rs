use anyhow::anyhow;
use cardano_tx_builder::{PlutusData, Signature};
/// Handles the map error
pub fn try_into_array<T: Copy, const N: usize>(v: &[T]) -> anyhow::Result<[T; N]> {
    <[T; N]>::try_from(v).map_err(|_err| anyhow!("Expected a Vec of length {}", N,))
}

pub fn signature_to_plutus_data(signature: Signature) -> PlutusData<'static> {
    PlutusData::from(signature.as_ref())
}

pub fn signature_from_plutus_data(plutus_data: &PlutusData) -> anyhow::Result<Signature> {
    Ok(Signature::from(<&[u8; 64]>::try_from(plutus_data)?.clone()))
}
