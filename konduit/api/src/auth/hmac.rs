//! HMAC-BLAKE3 session-token authentication.
//!
//! ## Flow
//!
//! 1. **Issuance** — client sends `POST /auth` with a CBOR body:
//!    ```text
//!    [[keytag_bytes, ttl_ms_u64], signature_bytes]
//!    ```
//!    where the Ed25519 signature covers
//!    ```text
//!    tbs = cbor(["KONDUIT_HMAC_ISSUE", server_pubkey, keytag, ttl_ms])
//!    ```
//!    The server verifies the signature, checks patron status, then returns a
//!    [`Token`] (CBOR-encoded) in the response body.
//!
//! 2. **Use** — client base64url-encodes the CBOR token bytes and sends them
//!    in the `konduit-hmac-token` header on every subsequent request.
//!
//! ## Why BLAKE3 keyed-hash?
//!
//! BLAKE3's keyed-hash mode is a native MAC primitive with the same security
//! as HMAC but faster.  The 32-byte key is the server's secret; the 32-byte
//! output is truncated to 20 bytes — more than enough to rule out brute-force
//! enumeration (the goal is anti-spam, not hiding secrets).

pub const FEATURE: &str = "auth.hmac";

#[cfg(feature = "actix")]
pub mod actix;
mod common;
pub mod error;

pub use common::{
    DOMAIN, HEADER_TOKEN, IssueRequest, MAC_LEN, Token, compute_mac, tbs_bytes, token_from_header,
    token_to_header, verify_issue_signature, verify_token,
};
pub use error::Error;
