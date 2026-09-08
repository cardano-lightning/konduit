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
    pub creates_pinned_channel: bool,
}

pub fn decode_signed_tx(bytes: &[u8]) -> anyhow::Result<SignedTx> {
    let (hash, ttl, creates_pinned_channel) = tx_details(bytes)?;
    let digest = Sha256::digest(bytes);
    Ok(SignedTx {
        hash,
        digest: digest.into(),
        ttl,
        bytes: bytes.to_vec(),
        creates_pinned_channel,
    })
}

fn tx_details(bytes: &[u8]) -> anyhow::Result<([u8; 32], Option<u64>, bool)> {
    let mut decoder = minicbor::Decoder::new(bytes);
    if let Ok(tx) = decoder.decode::<conway::MintedTx<'_>>()
        && decoder.position() == bytes.len()
    {
        return Ok((
            *Hasher::<256>::hash(tx.transaction_body.raw_cbor()),
            tx.transaction_body.ttl,
            tx.transaction_body
                .outputs
                .iter()
                .any(conway_output_is_pinned),
        ));
    }
    let mut decoder = minicbor::Decoder::new(bytes);
    if let Ok(tx) = decoder.decode::<babbage::MintedTx<'_>>()
        && decoder.position() == bytes.len()
    {
        return Ok((
            *Hasher::<256>::hash(tx.transaction_body.raw_cbor()),
            tx.transaction_body.ttl,
            tx.transaction_body
                .outputs
                .iter()
                .any(babbage_output_is_pinned),
        ));
    }
    Err(anyhow!("unable to decode signed transaction CBOR")).context("invalid transaction")
}

const PINNED_VALIDATOR: [u8; 28] = [
    0x8c, 0xc6, 0xbb, 0xae, 0xed, 0x22, 0xc2, 0x53, 0xb9, 0xd7, 0x03, 0xd3, 0x9f, 0x63, 0xb7, 0xe2,
    0x15, 0xf7, 0xaf, 0x08, 0xbd, 0xa9, 0x30, 0xac, 0x6b, 0x85, 0xeb, 0xaf,
];

fn address_is_pinned(address: &[u8]) -> bool {
    address.len() >= 29
        && matches!(address[0] >> 4, 1 | 3 | 5 | 7)
        && address[1..29] == PINNED_VALIDATOR
}

fn conway_output_is_pinned(output: &conway::MintedTransactionOutput<'_>) -> bool {
    match output {
        conway::PseudoTransactionOutput::Legacy(output) => address_is_pinned(&output.address),
        conway::PseudoTransactionOutput::PostAlonzo(output) => address_is_pinned(&output.address),
    }
}

fn babbage_output_is_pinned(output: &babbage::MintedTransactionOutput<'_>) -> bool {
    match output {
        babbage::PseudoTransactionOutput::Legacy(output) => address_is_pinned(&output.address),
        babbage::PseudoTransactionOutput::PostAlonzo(output) => address_is_pinned(&output.address),
    }
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
    if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(anyhow!("invalid uuid"));
    }
    let decoded = hex::decode(hex).context("uuid")?;
    decoded
        .as_slice()
        .try_into()
        .map_err(|_| anyhow!("invalid uuid"))
}

#[cfg(test)]
pub(crate) const TEST_TX: &str = "84a300d9010281825820c984c8bf52a141254c714c905b2d27b432d4b546f815fbc2fea7b9da6e490324030182a30058390082c1729d5fd44124a6ae72bcdb86b6e827aac6a74301e4003c092e6f4af57b0c9ff6ca5218967d1e7a3f572d7cd277d73468d3b2fca56572011a001092a803d818558203525101010023259800a518a4d136564004ae69a20058390082c1729d5fd44124a6ae72bcdb86b6e827aac6a74301e4003c092e6f4af57b0c9ff6ca5218967d1e7a3f572d7cd277d73468d3b2fca56572011a00a208bb021a00029755a0f5f6";

#[cfg(test)]
mod tests {
    use super::{TEST_TX, decode_signed_tx, parse_uuid};

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

    #[test]
    fn rejects_trailing_cbor() {
        let mut bytes = hex::decode(TEST_TX).unwrap();
        assert!(decode_signed_tx(&bytes).is_ok());
        bytes.push(0);
        assert!(decode_signed_tx(&bytes).is_err());
    }
}
