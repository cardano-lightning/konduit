//! Crypto boundary — all external hash and HMAC calls live here.
//!
//! ## Extension point
//! Implement [`HashHmac`] to bring your own crypto — useful for WASM bindings
//! (e.g. wrapping SubtleCrypto), alternative backends (`ring`, `aws-lc-rs`),
//! or test fakes. Pass it as the type parameter to [`Challenge::new`],
//! [`Challenge::verify`], and [`Challenge::solve`].
//!
//! ## Built-in implementations
//! - [`Sha2`]       — RustCrypto `sha2` crate; WASM-safe (feature `sha2`)
//! - [`Cryptoxide`] — `cryptoxide` crate; native, no std deps (feature `cryptoxide`)
//!
//! ## Adding a new algorithm
//! 1. Add a variant to [`PowScheme`].
//! 2. Add a match arm in each `HashHmac` impl.
//! 3. Add the dep behind a new feature flag.
//!
//! Nothing outside this file needs to change.

use crate::{Error, MAC_LEN, PowScheme};

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Cryptographic backend for proof-of-work operations.
///
/// Implement this to provide a custom hash/HMAC backend.
/// The two built-in impls are [`Sha2`] and [`Cryptoxide`].
pub trait HashHmac {
    /// Computes `hash(signature ‖ nonce)` for the puzzle.
    fn hash(scheme: PowScheme, signature: &[u8; MAC_LEN], nonce: u64) -> Result<[u8; 32], Error>;

    /// Computes `HMAC(data)` under `key` for challenge signing.
    fn hmac(scheme: PowScheme, key: &[u8; 32], data: &[u8]) -> Result<[u8; MAC_LEN], Error>;
}

// ---------------------------------------------------------------------------
// sha2 (RustCrypto) — WASM-safe
// ---------------------------------------------------------------------------

/// [`HashHmac`] implementation using the RustCrypto `sha2` crate.
///
/// WASM-safe. Enable with `features = ["sha2"]`.
#[cfg(feature = "sha2")]
pub struct Sha2;

#[cfg(feature = "sha2")]
impl HashHmac for Sha2 {
    fn hash(scheme: PowScheme, signature: &[u8; MAC_LEN], nonce: u64) -> Result<[u8; 32], Error> {
        match scheme {
            PowScheme::Sha256 => Ok(sha2_hash(signature, nonce)),
            #[allow(unreachable_patterns)]
            _ => Err(Error::Scheme),
        }
    }

    fn hmac(scheme: PowScheme, key: &[u8; 32], data: &[u8]) -> Result<[u8; MAC_LEN], Error> {
        match scheme {
            PowScheme::Sha256 => Ok(sha2_hmac(key, data)),
            #[allow(unreachable_patterns)]
            _ => Err(Error::Scheme),
        }
    }
}

#[cfg(feature = "sha2")]
fn sha2_hash(signature: &[u8; MAC_LEN], nonce: u64) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    Sha256::new()
        .chain_update(signature)
        .chain_update(nonce.to_be_bytes())
        .finalize()
        .into()
}

#[cfg(feature = "sha2")]
fn sha2_hmac(key: &[u8; 32], data: &[u8]) -> [u8; MAC_LEN] {
    use sha2::{Digest, Sha256};
    // Manual HMAC-SHA256. Key is always 32 bytes — fits in one 64-byte SHA256
    // block so no key pre-hashing is needed.
    const BLOCK: usize = 64;
    let mut ipad = [0x36u8; BLOCK];
    let mut opad = [0x5cu8; BLOCK];
    for i in 0..32 {
        ipad[i] ^= key[i];
        opad[i] ^= key[i];
    }
    let inner = Sha256::new()
        .chain_update(ipad)
        .chain_update(data)
        .finalize();
    Sha256::new()
        .chain_update(opad)
        .chain_update(inner)
        .finalize()
        .into()
}

// ---------------------------------------------------------------------------
// cryptoxide — native, no std deps
// ---------------------------------------------------------------------------

/// [`HashHmac`] implementation using the `cryptoxide` crate.
///
/// Native, no std deps. Enable with `features = ["cryptoxide"]`.
#[cfg(feature = "cryptoxide")]
pub struct Cryptoxide;

#[cfg(feature = "cryptoxide")]
impl HashHmac for Cryptoxide {
    fn hash(scheme: PowScheme, signature: &[u8; MAC_LEN], nonce: u64) -> Result<[u8; 32], Error> {
        match scheme {
            PowScheme::Sha256 => Ok(cryptoxide_hash(signature, nonce)),
            #[allow(unreachable_patterns)]
            _ => Err(Error::Scheme),
        }
    }

    fn hmac(scheme: PowScheme, key: &[u8; 32], data: &[u8]) -> Result<[u8; MAC_LEN], Error> {
        match scheme {
            PowScheme::Sha256 => Ok(cryptoxide_hmac(key, data)),
            #[allow(unreachable_patterns)]
            _ => Err(Error::Scheme),
        }
    }
}

#[cfg(feature = "cryptoxide")]
fn cryptoxide_hash(signature: &[u8; MAC_LEN], nonce: u64) -> [u8; 32] {
    use cryptoxide::digest::Digest;
    use cryptoxide::sha2::Sha256;
    let mut out = [0u8; 32];
    let mut h = Sha256::new();
    h.input(signature);
    h.input(&nonce.to_be_bytes());
    h.result(&mut out);
    out
}

#[cfg(feature = "cryptoxide")]
fn cryptoxide_hmac(key: &[u8; 32], data: &[u8]) -> [u8; MAC_LEN] {
    use cryptoxide::hmac::Hmac;
    use cryptoxide::mac::Mac;
    use cryptoxide::sha2::Sha256;
    let mut out = [0u8; MAC_LEN];
    let mut mac = Hmac::new(Sha256::new(), key);
    mac.input(data);
    mac.raw_result(&mut out);
    out
}
