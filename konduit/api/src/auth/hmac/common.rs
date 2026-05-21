use base64::Engine;
use cardano_sdk::VerificationKey;
use konduit_data::{Keytag, Signature, VerifyingKey};
use minicbor::Encoder;
use serde_with::serde_as;

use crate::auth::hmac::Error;

/// Domain separator burned into every TBS.  Fully valid CBOR text string inside
/// the outer array, so the TBS itself is valid CBOR and cannot collide with
/// cheques or squashes (which use integer indices, not string tags).
pub const DOMAIN: &str = "KONDUIT_HMAC_ISSUE";

/// HTTP header carrying the session token on authenticated requests.
pub const HEADER_TOKEN: &str = "konduit-hmac-token";

/// Length (bytes) of the truncated BLAKE3 MAC.  20 bytes = 160 bits — enough
/// to stop brute-force enumeration while keeping headers compact.
pub const MAC_LEN: usize = 20;

// ---------------------------------------------------------------------------
// IssueRequest — wire type for POST /auth
// ---------------------------------------------------------------------------

/// Body sent by the client to obtain a session token.
///
/// Wire encoding: `cbor([[keytag_bytes, ttl_ms_u64], signature_bytes])`
///
/// The client signs `tbs_bytes(server_pubkey, keytag, ttl_ms)` with their
/// Ed25519 key, then wraps the result as shown above.
pub struct IssueRequest {
    pub keytag: Keytag,
    /// Absolute expiry as Unix milliseconds.
    pub ttl_ms: u64,
    pub signature: Signature,
}

impl<'b, C> minicbor::Decode<'b, C> for IssueRequest {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        d.array()?; // outer 2-element array [inner, sig]
        d.array()?; // inner 2-element array [keytag, ttl]
        let keytag = Keytag::decode(d, ctx)?;
        let ttl_ms = d.u64()?;
        let signature = Signature::decode(d, ctx)?;
        Ok(Self {
            keytag,
            ttl_ms,
            signature,
        })
    }
}

// ---------------------------------------------------------------------------
// Token — self-describing session token
// ---------------------------------------------------------------------------

/// A session token returned by the server and re-sent by the client on every
/// subsequent request via the `konduit-hmac-token` header (base64url encoded).
///
/// Wire encoding: `cbor([keytag_bytes, ttl_ms_u64, mac_20bytes])`
#[serde_as]
#[derive(Clone, Debug, serde::Serialize)]
pub struct Token {
    pub keytag: Keytag,
    /// Absolute expiry as Unix milliseconds.
    pub ttl_ms: u64,
    #[serde_as(as = "serde_with::hex::Hex")]
    pub mac: [u8; MAC_LEN],
}

impl<C> minicbor::Encode<C> for Token {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.array(3)?;
        self.keytag.encode(e, ctx)?;
        e.u64(self.ttl_ms)?;
        e.bytes(&self.mac)?;
        Ok(())
    }
}

impl<'b, C> minicbor::Decode<'b, C> for Token {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        d.array()?;
        let keytag = Keytag::decode(d, ctx)?;
        let ttl_ms = d.u64()?;
        let mac_bytes = d.bytes()?;
        let mac: [u8; MAC_LEN] = mac_bytes
            .try_into()
            .map_err(|_| minicbor::decode::Error::message("hmac mac must be exactly 20 bytes"))?;
        Ok(Self {
            keytag,
            ttl_ms,
            mac,
        })
    }
}

// ---------------------------------------------------------------------------
// Core cryptographic operations
// ---------------------------------------------------------------------------

/// Constructs the canonical TBS (to-be-signed) bytes for token issuance.
///
/// ```text
/// tbs = cbor(["KONDUIT_HMAC_ISSUE", server_pubkey_bytes, keytag_bytes, ttl_ms_u64])
/// ```
///
/// Including `server_pubkey` prevents a signed message from being replayed
/// against a different server that shares the same HMAC key.
pub fn tbs_bytes(server_pubkey: &VerificationKey, keytag: &Keytag, ttl_ms: u64) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut e = Encoder::new(&mut buf);
    e.array(4)
        .expect("infallible")
        .str(DOMAIN)
        .expect("infallible")
        .bytes(server_pubkey.as_ref())
        .expect("infallible")
        .bytes(keytag.as_ref())
        .expect("infallible")
        .u64(ttl_ms)
        .expect("infallible");
    buf
}

/// Computes the 20-byte BLAKE3 keyed-hash MAC over the canonical TBS.
///
/// Uses BLAKE3's native keyed-hash mode (faster than HMAC-BLAKE3 wrapping and
/// carries the same PRF security guarantees with a 32-byte key).
pub fn compute_mac(
    hmac_key: &[u8; 32],
    server_pubkey: &VerificationKey,
    keytag: &Keytag,
    ttl_ms: u64,
) -> [u8; MAC_LEN] {
    let tbs = tbs_bytes(server_pubkey, keytag, ttl_ms);
    let hash = blake3::keyed_hash(hmac_key, &tbs);
    let mut mac = [0u8; MAC_LEN];
    mac.copy_from_slice(&hash.as_bytes()[..MAC_LEN]);
    mac
}

/// Verifies a token: TTL check then constant-time MAC comparison.
pub fn verify_token(
    hmac_key: &[u8; 32],
    server_pubkey: &VerificationKey,
    token: &Token,
    now_ms: u64,
) -> Result<(), Error> {
    if token.ttl_ms <= now_ms {
        return Err(Error::Expired);
    }
    let expected = compute_mac(hmac_key, server_pubkey, &token.keytag, token.ttl_ms);
    if !constant_time_eq(&expected, &token.mac) {
        return Err(Error::Unauthorized);
    }
    Ok(())
}

/// Verifies the Ed25519 signature in an [`IssueRequest`].
///
/// Extracts the client's [`VerifyingKey`] from the keytag, reconstructs the
/// canonical TBS, and delegates to `VerifyingKey::verify`.
pub fn verify_issue_signature(server_pubkey: &VerificationKey, req: &IssueRequest) -> bool {
    let (client_key, _tag): (VerifyingKey, _) = req.keytag.split();
    let tbs = tbs_bytes(server_pubkey, &req.keytag, req.ttl_ms);
    client_key.verify(&tbs, &req.signature)
}

// ---------------------------------------------------------------------------
// Header encoding / decoding
// ---------------------------------------------------------------------------

/// Encodes a [`Token`] as CBOR then base64url (no padding) for use in the
/// `konduit-hmac-token` HTTP header.
pub fn token_to_header(token: &Token) -> String {
    let bytes = minicbor::to_vec(token).expect("Token is always CBOR-encodable");
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// Decodes a [`Token`] from the base64url header value.
pub fn token_from_header(s: &str) -> Result<Token, Error> {
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(s)
        .map_err(|_| Error::BadToken)?;
    minicbor::decode(&bytes).map_err(|_| Error::BadToken)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn constant_time_eq(a: &[u8; MAC_LEN], b: &[u8; MAC_LEN]) -> bool {
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use cardano_sdk::VerificationKey;
    use konduit_data::Keytag;

    fn dummy_server_key() -> VerificationKey {
        VerificationKey::from([1u8; 32])
    }

    fn dummy_keytag() -> Keytag {
        Keytag::try_from(vec![0u8; 36]).unwrap()
    }

    fn dummy_hmac_key() -> [u8; 32] {
        [42u8; 32]
    }

    #[test]
    fn token_cbor_roundtrip() {
        let token = Token {
            keytag: dummy_keytag(),
            ttl_ms: 9_999_999_999_000,
            mac: [0xAB; MAC_LEN],
        };
        let bytes = minicbor::to_vec(&token).unwrap();
        let recovered: Token = minicbor::decode(&bytes).unwrap();
        assert_eq!(recovered.ttl_ms, token.ttl_ms);
        assert_eq!(recovered.mac, token.mac);
        assert_eq!(recovered.keytag.as_ref(), token.keytag.as_ref());
    }

    #[test]
    fn token_header_roundtrip() {
        let token = Token {
            keytag: dummy_keytag(),
            ttl_ms: 1_700_000_000_000,
            mac: [0x55; MAC_LEN],
        };
        let header = token_to_header(&token);
        let recovered = token_from_header(&header).unwrap();
        assert_eq!(recovered.ttl_ms, token.ttl_ms);
        assert_eq!(recovered.mac, token.mac);
    }

    #[test]
    fn verify_token_ok() {
        let key = dummy_hmac_key();
        let server_pubkey = dummy_server_key();
        let keytag = dummy_keytag();
        let ttl_ms = 2_000_000_000_000u64;
        let mac = compute_mac(&key, &server_pubkey, &keytag, ttl_ms);
        let token = Token {
            keytag,
            ttl_ms,
            mac,
        };
        assert!(verify_token(&key, &server_pubkey, &token, ttl_ms - 1).is_ok());
    }

    #[test]
    fn verify_token_expired() {
        let key = dummy_hmac_key();
        let server_pubkey = dummy_server_key();
        let keytag = dummy_keytag();
        let ttl_ms = 1_000u64;
        let mac = compute_mac(&key, &server_pubkey, &keytag, ttl_ms);
        let token = Token {
            keytag,
            ttl_ms,
            mac,
        };
        assert!(matches!(
            verify_token(&key, &server_pubkey, &token, ttl_ms + 1),
            Err(Error::Expired)
        ));
    }

    #[test]
    fn verify_token_bad_mac() {
        let key = dummy_hmac_key();
        let server_pubkey = dummy_server_key();
        let keytag = dummy_keytag();
        let ttl_ms = 2_000_000_000_000u64;
        let mut mac = compute_mac(&key, &server_pubkey, &keytag, ttl_ms);
        mac[0] ^= 0xFF;
        let token = Token {
            keytag,
            ttl_ms,
            mac,
        };
        assert!(matches!(
            verify_token(&key, &server_pubkey, &token, ttl_ms - 1),
            Err(Error::Unauthorized)
        ));
    }
}
