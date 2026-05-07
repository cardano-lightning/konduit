use cardano_sdk::{Signature, VerificationKey};
use konduit_data::Keytag;
use minicbor::Encoder;

/// The domain separator prefix for all PoP auth payloads.
/// Prepended *before* CBOR encoding to resist cross-protocol collision.
/// In other words, this should never look like a cheque or squash.
pub const DOMAIN: &[u8] = b"KONDUIT_AUTH";

///  Header name keytag
pub const HEADER_KEYTAG: &str = "Konduit-Keytag";
///  Header name signature
pub const HEADER_SIGNATURE: &str = "Konduit-Signature";

pub struct Headers {
    /// Carried in [`HEADER_KEYTAG`](`"Konduit-Keytag"`)
    pub keytag: Keytag,
    /// Carried in [`HEADER_SIGNATURE`](`"Konduit-Signature"`)
    pub signature: Signature,
}

/// Constructs the canonical byte payload the client must sign.
///
/// Structure: `DOMAIN || cbor([server_pubkey, keytag])`
///
/// The DOMAIN prefix is prepended raw (outside CBOR).
/// Some work is still required to verify that this can never take the form of cheque body or squash body.
pub fn to_bytes(server_public_key: &VerificationKey, keytag: &Keytag) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut encoder = Encoder::new(&mut buf);

    encoder
        .array(2)
        .expect("infallible")
        .bytes(server_public_key.as_ref())
        .expect("infallible")
        .bytes(keytag.as_ref())
        .expect("infallible");

    let mut payload = Vec::with_capacity(DOMAIN.len() + buf.len());
    payload.extend_from_slice(DOMAIN);
    payload.extend_from_slice(&buf);
    payload
}
