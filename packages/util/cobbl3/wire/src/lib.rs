//! # cobbl3-wire
//!
//! Wire types for the cobbl3 HMAC-BLAKE3 auth protocol.
//!
//! ## Crate responsibilities
//! - Define the wire shapes: [`Request`], [`Token`], [`Mac`], [`Response`]
//! - Define the [`Body`] trait: the contract between wire and server
//! - Provide [`Tbs`]: a conventional To-Be-Signed envelope (never sent over the wire)
//!
//! ## Non-responsibilities
//! - Key material, MAC computation, signature verification — `cobbl3-server`
//! - Domain strings, header names, MAC length — downstream `-wire` crate
//! - Crypto library choice — the [`Body`] implementor
//!
//! ## Design decisions
//!
//! ### `Body` trait
//! The only bound `cobbl3-wire` imposes on `B` beyond minicbor is [`Body::verify`].
//! `cobbl3-server` calls it without knowing what key `B` carries or what crypto
//! library backs it. The application owns those details entirely.
//!
//! ### `Tbs<B>` — To-Be-Signed envelope
//! Never sent over the wire. Both client and server construct it independently
//! from `B` to produce the bytes that are signed and verified. Shipped here so
//! both sides share the same construction. Downstream crates may use it, extend
//! it, or replace it — [`Body::tbs_bytes`] is the override point.
//!
//! ### `Mac<const N: usize>`
//! Generic over length so downstream crates fix `N` once in a type alias.
//! Constant-time `PartialEq` via `subtle` — timing-safe by default.
//!
//! ### `Token<B, N>` canonical string form
//! `Display` encodes to base64url (no padding). `FromStr` decodes.
//! Opaque by design — tokens live in HTTP headers, not human-readable contexts.
//!
//! ### What lives in the downstream `-wire` crate
//! ```rust,ignore
//! pub const MAC_LEN: usize = 20;
//! pub const DOMAIN:  &str  = "MYAPP_HMAC";
//! pub const HEADER:  &str  = "x-myapp-token";
//!
//! pub type AppMac   = cobbl3_wire::Mac<MAC_LEN>;
//! pub type AppToken = cobbl3_wire::Token<MyBody, MAC_LEN>;
//! ```

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use minicbor::{Decode, Encode};
use subtle::ConstantTimeEq;

// ---------------------------------------------------------------------------
// Body trait
// ---------------------------------------------------------------------------

/// Contract that any auth body must satisfy.
///
/// Implement [`verify`] with your chosen crypto library.
/// Override [`DOMAIN`] or [`tbs_bytes`] only if you need non-default behaviour.
///
/// [`verify`]: Body::verify
/// [`DOMAIN`]: Body::DOMAIN
/// [`tbs_bytes`]: Body::tbs_bytes
pub trait Body: for<'b> Encode<()> + for<'b> Decode<'b, ()> {
    /// Domain separator injected into [`Tbs`]. Override per application.
    const DOMAIN: &'static str = "COBBL3_HMAC";

    /// The canonical bytes that are signed and verified.
    /// Defaults to `Tbs::from_body(self).to_vec()`.
    /// Override if you need a different TBS structure.
    fn tbs_bytes(&self) -> Vec<u8>
    where
        Self: Sized,
    {
        Tbs::from_body(self).to_vec()
    }

    /// Verify `signature` over [`tbs_bytes`].
    /// The implementation supplies the key and crypto library — invisible to
    /// `cobbl3-wire` and `cobbl3-server`.
    ///
    /// [`tbs_bytes`]: Body::tbs_bytes
    fn verify(&self, signature: &[u8; 64]) -> bool;
}

// ---------------------------------------------------------------------------
// Tbs — To-Be-Signed
// ---------------------------------------------------------------------------

/// Conventional To-Be-Signed envelope. **Never sent over the wire.**
///
/// Wraps `B` with a domain separator so signed messages are namespaced and
/// cannot collide across protocols. Both client and server construct this
/// independently — its presence here ensures they agree on the structure.
///
/// Use [`Body::tbs_bytes`] as the override point rather than constructing
/// `Tbs` directly, so callers don't need to know the construction details.
#[derive(Debug, Clone, Encode)]
pub struct Tbs<'a, B> {
    /// Domain separator. Encoded as a CBOR text string so TBS is valid CBOR.
    #[n(0)]
    pub domain: &'a str,
    #[n(1)]
    pub body: &'a B,
}

impl<'a, B> Tbs<'a, B>
where
    B: Body,
{
    /// Construct from a body reference, using `B::DOMAIN`.
    pub fn from_body(body: &'a B) -> Self {
        Self {
            domain: B::DOMAIN,
            body,
        }
    }
}

impl<'a, B> Tbs<'a, B>
where
    B: for<'b> Encode<()>,
{
    /// Encode to bytes — these are the bytes that get signed and verified.
    pub fn to_vec(&self) -> Vec<u8> {
        minicbor::to_vec(self).expect("Tbs encoding is infallible")
    }
}

// ---------------------------------------------------------------------------
// Mac
// ---------------------------------------------------------------------------

/// A truncated BLAKE3 keyed-hash MAC of `N` bytes.
///
/// `N` is fixed by the downstream `-wire` crate via a type alias.
/// `PartialEq` is constant-time — timing-safe by construction.
#[derive(Debug, Clone, Encode, Decode)]
#[cbor(transparent)]
pub struct Mac<const N: usize>(#[n(0)] [u8; N]);

impl<const N: usize> Mac<N> {
    pub fn new(bytes: [u8; N]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; N] {
        &self.0
    }
}

impl<const N: usize> From<[u8; N]> for Mac<N> {
    fn from(bytes: [u8; N]) -> Self {
        Self(bytes)
    }
}

impl<const N: usize> PartialEq for Mac<N> {
    fn eq(&self, other: &Self) -> bool {
        self.0.ct_eq(&other.0).unwrap_u8() == 1
    }
}

impl<const N: usize> Eq for Mac<N> {}

// ---------------------------------------------------------------------------
// Request — client → server, POST /auth
// ---------------------------------------------------------------------------

/// Sent by the client to obtain a session [`Token`].
///
/// The client signs `body.tbs_bytes()` with their Ed25519 key and attaches
/// the 64-byte raw signature. The server calls `body.verify(&signature)`.
#[derive(Debug, Clone, Encode, Decode)]
pub struct Request<B> {
    #[n(0)]
    pub body: B,
    /// Raw 64-byte Ed25519 signature over `body.tbs_bytes()`.
    #[n(1)]
    pub signature: [u8; 64],
}

// ---------------------------------------------------------------------------
// Response — server → client, POST /auth
// ---------------------------------------------------------------------------

/// Server response to a successful [`Request`]: the session MAC.
/// The client wraps this in a [`Token`] for subsequent requests.
pub type Response<const N: usize> = Mac<N>;

// ---------------------------------------------------------------------------
// Token — client → server, subsequent requests
// ---------------------------------------------------------------------------

/// Session token issued by the server, carried by the client on every
/// subsequent request.
///
/// Canonical string form is base64url (no padding) via `Display` / `FromStr`.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct Token<B, const N: usize> {
    #[n(0)]
    pub body: B,
    #[n(1)]
    pub mac: Mac<N>,
}

/// Encodes to base64url (no padding) — the canonical header wire form.
impl<B, const N: usize> std::fmt::Display for Token<B, N>
where
    B: for<'b> Encode<()>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let bytes = minicbor::to_vec(self).expect("Token encoding is infallible");
        f.write_str(&URL_SAFE_NO_PAD.encode(bytes))
    }
}

/// Decodes from a base64url header value.
impl<B, const N: usize> std::str::FromStr for Token<B, N>
where
    for<'b> B: Decode<'b, ()>,
{
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Error> {
        let bytes = URL_SAFE_NO_PAD.decode(s).map_err(|_| Error::BadToken)?;
        minicbor::decode(&bytes).map_err(|_| Error::BadToken)
    }
}

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum Error {
    /// Token could not be decoded — malformed base64 or CBOR.
    BadToken,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::BadToken => f.write_str("token is malformed"),
        }
    }
}

impl std::error::Error for Error {}
