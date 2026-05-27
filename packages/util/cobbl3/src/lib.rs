//! # cobbl3
//!
//! Cobbl3 is a very simple HMAC-BLAKE3 auth protocol.
//!
//! ## Crate responsibilities
//! - Define the types [`Request`], [`Token`], [`Mac`], [`Response`]
//! - Define the [`Body`] trait: the contract between wire and server
//! - Provide [`Tbs`]: a conventional To-Be-Signed envelope (never sent over the wire)
//! - Provide HMAC_BLAKE3 versions of sign and verify (Both server)
//!
//! ## Design decisions
//!
//! ### `Body` trait
//! The only bound `cobbl3` imposes on `B` beyond minicbor is [`Body::verify`].
//! `cobbl3` calls it without knowing what key `B` carries or what crypto
//! library backs it. The application owns those details entirely.
//!
//! ### `Tbs<B>` - To-Be-Signed envelope
//! Never sent over the wire. Both client and server construct it independently
//! from `B` to produce the bytes that are signed and verified. Shipped here so
//! both sides share the same construction. Downstream crates may use it, extend
//! it, or replace it - [`Body::tbs_bytes`] is the override point.
//!
//! ### `Mac<const N: usize>`
//! Generic over length so downstream crates fix `N` once in a type alias.
//! Constant-time `PartialEq` via `subtle` - timing-safe by default.
//!
//! ### `Token<B, N>` canonical string form
//! `Display` encodes to base64url (no padding). `FromStr` decodes.
//! Opaque by design - tokens live in HTTP headers, not human-readable contexts.
//!
//! ### What lives in the downstream crate
//! ```rust,ignore
//! pub const MAC_LEN: usize = 20;
//! pub const DOMAIN:  &str  = "MYAPP_HMAC";
//! pub const HEADER:  &str  = "x-myapp-token";
//!
//! pub type AppMac   = cobbl3::Mac<MAC_LEN>;
//! pub type AppToken = cobbl3::Token<MyBody, MAC_LEN>;
//! ```
//! Also crypto library choice.

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
    /// The implementation supplies the key and crypto library
    ///
    /// [`tbs_bytes`]: Body::tbs_bytes
    fn verify(&self, signature: &[u8; 64]) -> bool;
}

// ---------------------------------------------------------------------------
// Tbs - To-Be-Signed
// ---------------------------------------------------------------------------

/// Conventional To-Be-Signed envelope. **Never sent over the wire.**
///
/// Wraps `B` with a domain separator so signed messages are namespaced and
/// cannot collide across protocols. Both client and server construct this
/// independently - its presence here ensures they agree on the structure.
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
    /// Encode to bytes - these are the bytes that get signed and verified.
    pub fn to_vec(&self) -> Vec<u8> {
        minicbor::to_vec(self).expect("Tbs encoding is infallible")
    }
}

// ---------------------------------------------------------------------------
// Mac
// ---------------------------------------------------------------------------

/// A truncated BLAKE3 keyed-hash MAC of `N` bytes.
///
/// `N` is fixed by the downstream crate via a type alias.
/// `PartialEq` is constant-time - timing-safe by construction.
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
// Request - client → server, POST /auth
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
// Response - server → client, POST /auth
// ---------------------------------------------------------------------------

/// Server response to a successful [`Request`]: the session MAC.
/// The client wraps this in a [`Token`] for subsequent requests.
pub type Response<const N: usize> = Mac<N>;

// ---------------------------------------------------------------------------
// Token - client → server, subsequent requests
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

/// Encodes to base64url (no padding) - the canonical header wire form.
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
        let bytes = URL_SAFE_NO_PAD.decode(s).map_err(|_| Error::Token)?;
        minicbor::decode(&bytes).map_err(|_| Error::Token)
    }
}

#[cfg(feature = "server")]
#[derive(Debug, Clone, Encode, Decode)]
#[repr(transparent)]
#[cbor(transparent)]
pub struct HmacKey(#[n(0)] [u8; 32]);

#[cfg(feature = "server")]
impl From<[u8; 32]> for HmacKey {
    fn from(value: [u8; 32]) -> Self {
        Self(value)
    }
}

#[cfg(feature = "server")]
impl HmacKey {
    pub fn issue<B: Body, const N: usize>(
        &self,
        body: &B,
        signature: &[u8; 64],
    ) -> Result<Mac<N>, Error> {
        if !body.verify(signature) {
            return Err(Error::ClientSignature);
        }
        Ok(self.sign(body))
    }

    pub fn sign<B: Body, const N: usize>(&self, body: &B) -> Mac<N> {
        let hash = blake3::keyed_hash(&self.0, &minicbor::to_vec(body).expect("Infallible"));
        let mut mac = [0u8; N];
        mac.copy_from_slice(&hash.as_bytes()[..N]);
        mac.into()
    }

    pub fn verify<B: Body, const N: usize>(&self, token: &Token<B, N>) -> Result<(), Error> {
        let expected = self.sign(&token.body);
        if expected != token.mac {
            return Err(Error::HmacSignature);
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
#[cfg_attr(
    feature = "problem-details",
    derive(problem_details_wire::ProblemDetail)
)]
pub enum Error {
    /// Client signature could not be verified.
    #[error("client signature could not be verified")]
    #[cfg_attr(
        feature = "problem-details",
        problem(
            slug = "client-signature-invalid",
            title = "Client Signature Invalid",
            http_status = 401
        )
    )]
    ClientSignature,

    /// Token could not be decoded. Malformed base64 or CBOR.
    #[error("token is malformed")]
    #[cfg_attr(
        feature = "problem-details",
        problem(slug = "token-malformed", title = "Token Malformed", http_status = 400)
    )]
    Token,

    /// HMAC signature does not match.
    #[error("failed to verify signature")]
    #[cfg_attr(
        feature = "problem-details",
        problem(
            slug = "hmac-signature-invalid",
            title = "HMAC Signature Invalid",
            http_status = 401
        )
    )]
    HmacSignature,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> HmacKey {
        HmacKey::from([0u8; 32])
    }

    struct TestBody {
        nonce: u64,
    }

    // minimal Body impl — verify always passes so we can test HmacKey in isolation
    impl Body for TestBody {
        fn verify(&self, _signature: &[u8; 64]) -> bool {
            true
        }
    }

    impl minicbor::Encode<()> for TestBody {
        fn encode<W: minicbor::encode::Write>(
            &self,
            e: &mut minicbor::Encoder<W>,
            _: &mut (),
        ) -> Result<(), minicbor::encode::Error<W::Error>> {
            e.u64(self.nonce)?.ok()
        }
    }

    impl<'b> minicbor::Decode<'b, ()> for TestBody {
        fn decode(
            d: &mut minicbor::Decoder<'b>,
            _: &mut (),
        ) -> Result<Self, minicbor::decode::Error> {
            Ok(Self { nonce: d.u64()? })
        }
    }

    #[test]
    fn issue_and_verify_roundtrip() {
        let key = key();
        let body = TestBody { nonce: 1 };
        let mac: Mac<20> = key.issue(&body, &[0u8; 64]).unwrap();
        let token = Token { body, mac };
        assert!(key.verify(&token).is_ok());
    }

    #[test]
    fn tampered_body_fails_verify() {
        let key = key();
        let mac: Mac<20> = key.sign(&TestBody { nonce: 1 });
        let token = Token {
            body: TestBody { nonce: 2 },
            mac,
        }; // body swapped
        assert!(key.verify(&token).is_err());
    }

    #[test]
    fn different_key_fails_verify() {
        let mac: Mac<20> = key().sign(&TestBody { nonce: 1 });
        let token = Token {
            body: TestBody { nonce: 1 },
            mac,
        };
        assert!(HmacKey::from([1u8; 32]).verify(&token).is_err());
    }

    #[test]
    fn issue_rejects_bad_client_signature() {
        #[derive(Encode, Decode)]
        struct RejectBody(#[n(0)] u64);

        impl Body for RejectBody {
            fn verify(&self, _: &[u8; 64]) -> bool {
                false
            }
        }
        assert!(matches!(
            key().issue::<RejectBody, 20>(&RejectBody(42), &[0u8; 64]),
            Err(Error::ClientSignature)
        ));
    }

    #[test]
    fn token_display_roundtrip() {
        let key = key();
        let mac: Mac<20> = key.sign(&TestBody { nonce: 42 });
        let token = Token {
            body: TestBody { nonce: 42 },
            mac,
        };
        let s = token.to_string();
        let decoded: Token<TestBody, 20> = s.parse().unwrap();
        assert_eq!(decoded.body.nonce, 42);
    }

    #[cfg(feature = "problem-details")]
    mod problem_details {
        use super::*;
        use problem_details_wire::ProblemDetail;

        #[test]
        fn slugs_and_statuses() {
            assert_eq!(Error::ClientSignature.slug(), "client-signature-invalid");
            assert_eq!(Error::ClientSignature.http_status(), 401);
            assert_eq!(Error::Token.slug(), "token-malformed");
            assert_eq!(Error::Token.http_status(), 400);
            assert_eq!(Error::HmacSignature.slug(), "hmac-signature-invalid");
            assert_eq!(Error::HmacSignature.http_status(), 401);
        }
    }
}
