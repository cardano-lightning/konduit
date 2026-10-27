use crate::Blake2b256;
use reqwest::header::{HeaderMap, HeaderValue};

/// A response from a Kupo endpoint: the decoded body plus the two
/// "context" headers Kupo sets on every response.
///
/// From [Kupo's docs][kupo-docs]:
///
/// > First, any results from Kupo are delivered with a timestamp
/// > `created_at`, which gives you the absolute slot number at which an
/// > entry has been created and the block header hash that contains the
/// > associated output.
/// >
/// > Second, any response from the server contains some practical header
/// > fields:
/// >   - `X-Most-Recent-Checkpoint`: an absolute slot number of the most
/// >     recent block indexed by Kupo at the moment of the request.
/// >   - `ETag`: a hex-encoded block header hash digest of that same most
/// >     recent block.
///
/// [kupo-docs]: https://cardanosolutions.github.io/kupo/#section/Rollbacks-and-Headers
#[derive(Debug, Clone)]
pub struct KupoResponse<T> {
    /// The decoded JSON body.
    pub body: T,

    /// `X-Most-Recent-Checkpoint` from the response, parsed as a slot
    /// number. `None` if the header was missing or not a valid number.
    pub most_recent_checkpoint: Option<u64>,

    /// `ETag` from the response, decoded as a 32-byte block-header hash.
    /// `None` if the header was missing or malformed.
    pub etag: Option<Blake2b256>,
}

impl<T> KupoResponse<T> {
    /// Build a `KupoResponse` from a parsed body and the response headers.
    ///
    /// Both headers are best-effort: if a header is missing or malformed
    /// the corresponding field is `None`.
    pub fn from_parts(body: T, headers: &HeaderMap) -> Self {
        let most_recent_checkpoint = headers
            .get("x-most-recent-checkpoint")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.trim().parse::<u64>().ok());

        let etag = headers.get("etag").and_then(parse_etag);

        Self {
            body,
            most_recent_checkpoint,
            etag,
        }
    }

    /// Project only the body, dropping the headers.
    pub fn into_body(self) -> T {
        self.body
    }
}

/// Decode an HTTP `ETag` header as Kupo's block-header hash, accepting
/// surrounding quotes and an optional weak-validator prefix.
fn parse_etag(value: &HeaderValue) -> Option<Blake2b256> {
    value.to_str().ok().and_then(|s| {
        let s = s.trim();
        let s = s.strip_prefix("W/").unwrap_or(s);
        s.trim_matches('"').parse().ok()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::{HeaderMap, HeaderValue};

    fn headers(pairs: &[(&'static str, &'static str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in pairs {
            h.insert(*k, HeaderValue::from_str(v).unwrap());
        }
        h
    }

    #[test]
    fn parses_both_headers() {
        let etag = "ab".repeat(32);
        let quoted_etag = format!("\"{etag}\"");
        let mut h = HeaderMap::new();
        h.insert("x-most-recent-checkpoint", HeaderValue::from_static("42"));
        h.insert("etag", HeaderValue::from_str(&quoted_etag).unwrap());
        let r = KupoResponse::<()>::from_parts((), &h);
        assert_eq!(r.most_recent_checkpoint, Some(42));
        assert_eq!(r.etag, Some(Blake2b256([0xab; 32])));
    }

    #[test]
    fn strips_weak_validator_prefix() {
        let etag = format!("W/\"{}\"", "ab".repeat(32));
        let mut h = HeaderMap::new();
        h.insert("etag", HeaderValue::from_str(&etag).unwrap());
        let r = KupoResponse::<()>::from_parts((), &h);
        assert_eq!(r.etag, Some(Blake2b256([0xab; 32])));
    }

    #[test]
    fn missing_headers_yield_none() {
        let h = HeaderMap::new();
        let r = KupoResponse::<()>::from_parts((), &h);
        assert!(r.most_recent_checkpoint.is_none());
        assert!(r.etag.is_none());
    }

    #[test]
    fn malformed_checkpoint_yields_none() {
        let h = headers(&[("x-most-recent-checkpoint", "not-a-number")]);
        let r = KupoResponse::<()>::from_parts((), &h);
        assert!(r.most_recent_checkpoint.is_none());
    }
}
