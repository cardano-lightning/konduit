use anyhow::{Context, anyhow};
use pallas_codec::minicbor;
use pallas_crypto::hash::Hasher;
use pallas_primitives::{babbage, conway};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone)]
pub struct SignedTx {
    pub hash: [u8; 32],
    pub digest: [u8; 32],
    pub ttl: Option<u64>,
    pub bytes: Vec<u8>,
}

pub fn decode_signed_tx(bytes: &[u8]) -> anyhow::Result<SignedTx> {
    let hash = tx_body_hash(bytes)?;
    let ttl = tx_ttl(bytes);
    let digest = Sha256::digest(bytes);
    Ok(SignedTx {
        hash,
        digest: digest.into(),
        ttl,
        bytes: bytes.to_vec(),
    })
}

fn tx_body_hash(bytes: &[u8]) -> anyhow::Result<[u8; 32]> {
    if let Ok(tx) = minicbor::decode::<conway::MintedTx<'_>>(bytes) {
        return Ok(*Hasher::<256>::hash(tx.transaction_body.raw_cbor()));
    }
    if let Ok(tx) = minicbor::decode::<babbage::MintedTx<'_>>(bytes) {
        return Ok(*Hasher::<256>::hash(tx.transaction_body.raw_cbor()));
    }
    Err(anyhow!("unable to decode signed transaction CBOR")).context("invalid transaction")
}

fn tx_ttl(bytes: &[u8]) -> Option<u64> {
    if let Ok(tx) = minicbor::decode::<conway::MintedTx<'_>>(bytes) {
        return tx.transaction_body.ttl;
    }
    if let Ok(tx) = minicbor::decode::<babbage::MintedTx<'_>>(bytes) {
        return tx.transaction_body.ttl;
    }
    None
}

pub fn parse_lowercase_hex(input: &str) -> anyhow::Result<Vec<u8>> {
    if input.is_empty()
        || !input
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
    {
        return Err(anyhow!("expected lowercase hex"));
    }
    hex::decode(input).context("hex")
}

pub fn parse_tx_id(input: &str) -> anyhow::Result<[u8; 32]> {
    let bytes = parse_lowercase_hex(input)?;
    bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow!("transaction id must be 32 bytes"))
}

pub fn parse_uuid(input: &str) -> anyhow::Result<[u8; 16]> {
    let parts: Vec<&str> = input.split('-').collect();
    if parts.len() != 5
        || parts[0].len() != 8
        || parts[1].len() != 4
        || parts[2].len() != 4
        || parts[3].len() != 4
        || parts[4].len() != 12
    {
        return Err(anyhow!("invalid uuid"));
    }
    let hex: String = parts.concat();
    if !hex
        .bytes()
        .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F'))
    {
        return Err(anyhow!("invalid uuid"));
    }
    let decoded = hex::decode(hex).context("uuid")?;
    decoded
        .as_slice()
        .try_into()
        .map_err(|_| anyhow!("invalid uuid"))
}

#[cfg(test)]
mod tests {
    use super::parse_uuid;

    #[test]
    fn parse_uuid_canonical() {
        let id = parse_uuid("550e8400-e29b-41d4-a716-446655440000").expect("uuid");
        assert_eq!(id[0], 0x55);
        assert_eq!(id[15], 0x00);
    }

    #[test]
    fn parse_uuid_rejects_shape() {
        assert!(parse_uuid("550e8400e29b41d4a716446655440000").is_err());
    }
}
