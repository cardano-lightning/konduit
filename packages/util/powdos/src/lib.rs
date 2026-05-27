//! # powdos
//!
//! Crypto-agnostic proof-of-work challenge — issue, solve, verify.
//!
//! ## Protocol
//! ```text
//! Server  →  Challenge { scheme, difficulty, expires_at, mac }
//! Client  →  brute-forces nonce where hash(mac ‖ nonce) has ≥ difficulty leading zero bits
//! Client  →  submits nonce
//! Server  →  re-derives mac, checks expiry, checks nonce hash
//! ```
//!
//! ## MAC
//! `HMAC(scheme ‖ difficulty ‖ expires_at ‖ client_ip)` keyed with the server
//! secret. Binds challenge parameters to a specific client so challenges cannot
//! be transferred between clients.
//!
//! ## Time
//! All timestamps and durations are in **milliseconds** since the Unix epoch.
//!
//! ## Wire format
//! [`Challenge`] encodes with `minicbor` (CBOR) and `serde` (JSON, mac hex-encoded).
//!
//! ## Feature flags
//!
//! ### Role
//! - `server` — enables [`Challenge::new`] and [`Challenge::verify`]
//! - `client` — enables [`Challenge::solve`]
//!
//! ### Crypto
//! All crypto calls are dispatched through the [`HashHmac`] trait. Pass your
//! own implementation or use a built-in:
//! - `sha2`       — [`Sha2`] impl; RustCrypto, WASM-safe
//! - `cryptoxide` — [`Cryptoxide`] impl; native, no std deps
//!
//! Without a crypto feature you can still provide your own [`HashHmac`] impl.
//! If no implementation covers the requested scheme, [`Error::Scheme`] is returned.
//!
//! Typical combinations:
//! ```toml
//! # WASM client
//! features = ["client", "sha2"]
//! # Native server, single crypto dep
//! features = ["server", "cryptoxide"]
//! # Native server + client, single crypto dep
//! features = ["server", "client", "cryptoxide"]
//! ```
//!
//! ## Algorithm extensibility
//! [`PowScheme`] is `#[non_exhaustive]`. New variants are non-breaking on the
//! wire; [`HashHmac`] implementors will fail to compile until their match arms
//! are updated.

use core::fmt;
use std::str::FromStr;

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

mod crypto;
mod error;

pub use crypto::HashHmac;
pub use error::Error;

#[cfg(feature = "sha2")]
pub use crypto::Sha2;

#[cfg(feature = "cryptoxide")]
pub use crypto::Cryptoxide;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Byte length of the HMAC challenge MAC.
pub const MAC_LEN: usize = 32;

// ---------------------------------------------------------------------------
// PowScheme
// ---------------------------------------------------------------------------

/// Hash algorithm used for both the puzzle hash and the HMAC.
///
/// `#[non_exhaustive]` — new variants may be added in future minor versions.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Encode, Decode, PartialEq, Eq)]
#[non_exhaustive]
pub enum PowScheme {
    #[n(0)]
    Sha256,
}

// ---------------------------------------------------------------------------
// Challenge
// ---------------------------------------------------------------------------

/// A proof-of-work challenge issued by the server.
///
/// Wire type: CBOR via `minicbor`, JSON via `serde` (mac hex-encoded).
///
/// All methods are generic over a [`HashHmac`] backend `C`. Use a built-in
/// ([`Sha2`], [`Cryptoxide`]) or provide your own.
#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize, Encode, Decode)]
pub struct Challenge {
    /// Hash algorithm for the puzzle and HMAC.
    #[n(0)]
    pub scheme: PowScheme,
    /// Required leading zero bits in `hash(mac ‖ nonce)`.
    #[n(1)]
    pub difficulty: u8,
    /// Unix timestamp (milliseconds) after which this challenge is invalid.
    #[n(2)]
    pub expires_at: u64,
    /// `HMAC(scheme ‖ difficulty ‖ expires_at ‖ client_ip)` under server secret.
    #[n(3)]
    #[serde_as(as = "serde_with::hex::Hex")]
    pub mac: [u8; MAC_LEN],
}

impl fmt::Display for Challenge {
    // Not human-readable — encodes as base64url(CBOR).
    // Canonical wire form for HTTP headers (e.g. Pow-Challenge).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bytes = minicbor::to_vec(self).expect("Challenge encoding is infallible");
        f.write_str(&URL_SAFE_NO_PAD.encode(bytes))
    }
}

impl FromStr for Challenge {
    type Err = Error;

    // Inverse of Display — decodes base64url(CBOR).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = URL_SAFE_NO_PAD.decode(s).map_err(|_| Error::Parse)?;
        minicbor::decode(&bytes).map_err(|_| Error::Parse)
    }
}

// ---------------------------------------------------------------------------
// Server: issue + verify
// ---------------------------------------------------------------------------

#[cfg(feature = "server")]
impl Challenge {
    /// Issues a new challenge bound to `client_ip`, expiring after `ttl` milliseconds.
    ///
    /// Returns [`Error::Scheme`] if `C` does not support `scheme`.
    pub fn new<C: HashHmac>(
        scheme: PowScheme,
        difficulty: u8,
        client_ip: &str,
        server_secret: &[u8; 32],
        ttl: u64,
    ) -> Result<Self, Error> {
        let expires_at = now().saturating_add(ttl);
        let mac = mac::<C>(scheme, server_secret, difficulty, expires_at, client_ip)?;
        Ok(Challenge {
            scheme,
            difficulty,
            expires_at,
            mac,
        })
    }

    /// Verifies a client-submitted nonce against this challenge.
    ///
    /// Checks in order:
    /// 1. Challenge has not expired → [`Error::Time`]
    /// 2. Signature valid for `client_ip` (constant-time) → [`Error::Mac`]
    /// 3. `hash(mac ‖ nonce)` meets difficulty → [`Error::Hash`]
    /// 4. Crypto available for scheme → [`Error::Scheme`]
    pub fn verify<C: HashHmac>(
        &self,
        client_ip: &str,
        server_secret: &[u8; 32],
        nonce: u64,
    ) -> Result<(), Error> {
        use subtle::ConstantTimeEq;

        if now() > self.expires_at {
            return Err(Error::Time);
        }

        let expected = mac::<C>(
            self.scheme,
            server_secret,
            self.difficulty,
            self.expires_at,
            client_ip,
        )?;

        if self.mac.ct_eq(&expected).unwrap_u8() != 1 {
            return Err(Error::Mac);
        }

        let h = C::hash(self.scheme, &self.mac, nonce)?;
        if leading_zero_bits(&h) < self.difficulty {
            return Err(Error::Hash);
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Client: solve
// ---------------------------------------------------------------------------

#[cfg(feature = "client")]
impl Challenge {
    /// Brute-forces a nonce satisfying the proof-of-work puzzle.
    ///
    /// Iterates from 0 until `hash(mac ‖ nonce)` has ≥ `difficulty`
    /// leading zero bits. Expected iterations: `2^difficulty`.
    ///
    /// Returns [`Error::Scheme`] if `C` does not support `scheme`.
    pub fn solve<C: HashHmac>(&self) -> Result<u64, Error> {
        let mut nonce = 0u64;
        loop {
            let h = C::hash(self.scheme, &self.mac, nonce)?;
            if leading_zero_bits(&h) >= self.difficulty {
                return Ok(nonce);
            }
            nonce += 1;
        }
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Computes the HMAC MAC authenticating challenge parameters.
///
/// Keyed over `(scheme, difficulty, expires_at, client_ip)` — any mutation
/// of these fields invalidates the MAC.
#[cfg(feature = "server")]
fn mac<C: HashHmac>(
    scheme: PowScheme,
    server_secret: &[u8; 32],
    difficulty: u8,
    expires_at: u64,
    client_ip: &str,
) -> Result<[u8; MAC_LEN], Error> {
    let mut data = Vec::with_capacity(1 + 1 + 8 + client_ip.len());
    data.extend_from_slice(&(scheme as u8).to_be_bytes());
    data.push(difficulty);
    data.extend_from_slice(&expires_at.to_be_bytes());
    data.extend_from_slice(client_ip.as_bytes());
    C::hmac(scheme, server_secret, &data)
}

#[cfg(any(feature = "server", feature = "client"))]
fn leading_zero_bits(data: &[u8]) -> u8 {
    let mut count = 0u8;
    for byte in data {
        if *byte == 0 {
            count += 8;
        } else {
            count += byte.leading_zeros() as u8;
            break;
        }
    }
    count
}

#[cfg(feature = "server")]
fn now() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before Unix epoch")
        .as_millis() as u64
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn challenge_roundtrip_base64() {
    let c = Challenge {
        scheme: PowScheme::Sha256,
        difficulty: 8,
        expires_at: 1_748_000_000_000,
        mac: [32u8; MAC_LEN],
    };
    let encoded = c.to_string();
    println!("{}", encoded);
    let decoded: Challenge = encoded.parse().unwrap();
    assert_eq!(c.scheme, decoded.scheme);
    assert_eq!(c.difficulty, decoded.difficulty);
    assert_eq!(c.expires_at, decoded.expires_at);
    assert_eq!(c.mac, decoded.mac);
}

#[cfg(all(
    test,
    feature = "server",
    feature = "client",
    any(feature = "sha2", feature = "cryptoxide")
))]
mod tests {
    use super::*;

    const SECRET: &[u8; 32] = b"super_secret_key_32_bytes_long!!";
    const IP: &str = "127.0.0.1";

    fn do_solve_then_verify<C: HashHmac>(c: Challenge) {
        let nonce = c.solve::<C>().unwrap();
        assert!(c.verify::<C>(IP, SECRET, nonce).is_ok());
    }

    fn do_wrong_nonce_rejected<C: HashHmac>(c: Challenge) {
        let nonce = c.solve::<C>().unwrap();
        assert!(matches!(
            c.verify::<C>(IP, SECRET, nonce + 1),
            Err(Error::Hash)
        ));
    }

    fn do_wrong_ip_rejected<C: HashHmac>(c: Challenge) {
        let nonce = c.solve::<C>().unwrap();
        assert!(matches!(
            c.verify::<C>("10.0.0.1", SECRET, nonce),
            Err(Error::Mac)
        ));
    }

    fn do_expired_rejected<C: HashHmac>() {
        let c = Challenge {
            scheme: PowScheme::Sha256,
            difficulty: 4,
            expires_at: 1,
            mac: [0u8; MAC_LEN],
        };
        assert!(matches!(c.verify::<C>(IP, SECRET, 0), Err(Error::Time)));
    }

    #[cfg(feature = "sha2")]
    mod sha2 {
        use super::*;

        fn challenge(difficulty: u8) -> Challenge {
            Challenge::new::<Sha2>(PowScheme::Sha256, difficulty, IP, SECRET, 60_000).unwrap()
        }

        #[test]
        fn solve_then_verify() {
            do_solve_then_verify::<Sha2>(challenge(8));
        }
        #[test]
        fn wrong_nonce_rejected() {
            do_wrong_nonce_rejected::<Sha2>(challenge(8));
        }
        #[test]
        fn wrong_ip_rejected() {
            do_wrong_ip_rejected::<Sha2>(challenge(8));
        }
        #[test]
        fn expired_rejected() {
            do_expired_rejected::<Sha2>();
        }
    }

    #[cfg(feature = "cryptoxide")]
    mod cryptoxide {
        use super::*;

        fn challenge(difficulty: u8) -> Challenge {
            Challenge::new::<Cryptoxide>(PowScheme::Sha256, difficulty, IP, SECRET, 60_000).unwrap()
        }

        #[test]
        fn solve_then_verify() {
            do_solve_then_verify::<Cryptoxide>(challenge(8));
        }
        #[test]
        fn wrong_nonce_rejected() {
            do_wrong_nonce_rejected::<Cryptoxide>(challenge(8));
        }
        #[test]
        fn wrong_ip_rejected() {
            do_wrong_ip_rejected::<Cryptoxide>(challenge(8));
        }
        #[test]
        fn expired_rejected() {
            do_expired_rejected::<Cryptoxide>();
        }
    }
}
