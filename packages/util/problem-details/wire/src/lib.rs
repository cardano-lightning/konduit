#![doc = include_str!("../README.md")]

pub use problem_details_derive::ProblemDetail;

// ── Wire type ─────────────────────────────────────────────────────────────────

#[derive(
    Debug, Clone, serde::Serialize, serde::Deserialize, minicbor::Encode, minicbor::Decode,
)]
pub struct ProblemDetailBody {
    #[n(0)]
    pub r#type: String,
    #[n(1)]
    pub title: String,
    #[n(2)]
    pub status: u16,
    #[n(3)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[n(4)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,
}

// ── Trait ─────────────────────────────────────────────────────────────────────

/// Implement via `#[derive(ProblemDetail)]`. See derive macro docs for usage.
pub trait ProblemDetail {
    /// Short stable identifier, e.g. `"not-a-patron"`. Used as the final
    /// segment of the `type` URI, and directly by gRPC (`ErrorInfo.reason`).
    fn slug(&self) -> &'static str;
    fn problem_type(&self) -> &'static str;
    fn title(&self) -> &'static str;
    fn http_status(&self) -> u16;
    fn detail(&self) -> Option<String> {
        None
    }
    fn instance(&self) -> Option<String> {
        None
    }

    fn to_body(&self) -> ProblemDetailBody {
        ProblemDetailBody {
            r#type: self.problem_type().to_owned(),
            title: self.title().to_owned(),
            status: self.http_status(),
            detail: self.detail(),
            instance: self.instance(),
        }
    }

    fn to_cbor(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        minicbor::encode(self.to_body(), &mut buf).expect("infallible");
        buf
    }

    fn to_json(&self) -> Vec<u8> {
        serde_json::to_vec(&self.to_body()).expect("infallible")
    }
}

// ── WithDetail ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct WithDetail<E> {
    pub error: E,
    pub detail: String,
    pub instance: Option<String>,
}

impl<E: ProblemDetail> ProblemDetail for WithDetail<E> {
    fn slug(&self) -> &'static str {
        self.error.slug()
    }
    fn problem_type(&self) -> &'static str {
        self.error.problem_type()
    }
    fn title(&self) -> &'static str {
        self.error.title()
    }
    fn http_status(&self) -> u16 {
        self.error.http_status()
    }
    fn detail(&self) -> Option<String> {
        Some(self.detail.clone())
    }
    fn instance(&self) -> Option<String> {
        self.instance.clone()
    }
}

// ── ProblemDetailExt ──────────────────────────────────────────────────────────

pub trait ProblemDetailExt: ProblemDetail + Sized {
    fn with_detail(self, detail: impl Into<String>) -> WithDetail<Self> {
        WithDetail {
            error: self,
            detail: detail.into(),
            instance: None,
        }
    }
    fn with_instance(
        self,
        detail: impl Into<String>,
        instance: impl Into<String>,
    ) -> WithDetail<Self> {
        WithDetail {
            error: self,
            detail: detail.into(),
            instance: Some(instance.into()),
        }
    }
}

impl<E: ProblemDetail> ProblemDetailExt for E {}

// ── Content-Type constants ────────────────────────────────────────────────────

pub const CONTENT_TYPE_CBOR: &str = "application/problem+cbor";
pub const CONTENT_TYPE_JSON: &str = "application/problem+json";

// ── Re-export magic        ────────────────────────────────────────────────────

#[doc(hidden)]
pub extern crate self as problem_details_wire;

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, ProblemDetail)]
    enum Error {
        /// MAC verification failed.
        #[problem(slug = "unauthorized", title = "Unauthorized", http_status = 401)]
        Unauthorized,

        /// Keytag is not a recognised patron.
        #[problem(slug = "not-a-patron", title = "Not a Patron", http_status = 403)]
        NotPatron,
    }

    // ── Derive output ─────────────────────────────────────────────────────────────

    #[test]
    fn slug() {
        assert_eq!(Error::Unauthorized.slug(), "unauthorized");
        assert_eq!(Error::NotPatron.slug(), "not-a-patron");
    }

    #[test]
    fn title() {
        assert_eq!(Error::Unauthorized.title(), "Unauthorized");
        assert_eq!(Error::NotPatron.title(), "Not a Patron");
    }

    #[test]
    fn http_status() {
        assert_eq!(Error::Unauthorized.http_status(), 401);
        assert_eq!(Error::NotPatron.http_status(), 403);
    }

    #[test]
    fn problem_type_uri() {
        // PROBLEM_DETAIL_BASE_URL is set in .cargo/config.toml
        let uri = Error::Unauthorized.problem_type();
        assert!(uri.starts_with("https://konduit.channel/errors/"));
        assert!(uri.ends_with("/unauthorized"));
    }

    // ── WithDetail ───────────────────────────────────────────────────────────────

    #[test]
    fn with_detail_overides() {
        let e = Error::Unauthorized.with_detail("clock skew was 42s");
        assert_eq!(e.slug(), "unauthorized");
        assert_eq!(e.http_status(), 401);
        assert_eq!(e.detail(), Some("clock skew was 42s".to_owned()));
    }

    #[test]
    fn with_instance_sets_both() {
        let e = Error::NotPatron.with_instance("keytag abc has no channel", "/requests/99");
        assert_eq!(e.detail(), Some("keytag abc has no channel".to_owned()));
        assert_eq!(e.instance(), Some("/requests/99".to_owned()));
    }

    #[test]
    fn base_error_has_no_detail() {
        assert_eq!(Error::Unauthorized.detail(), None);
    }

    // ── CBOR round-trip ───────────────────────────────────────────────────────────

    #[test]
    fn cbor_round_trip() {
        let original = Error::NotPatron.with_detail("no channel on record");
        let bytes = original.to_cbor();
        let decoded: ProblemDetailBody = minicbor::decode(&bytes).expect("decode failed");

        assert_eq!(decoded.title, "Not a Patron");
        assert_eq!(decoded.status, 403);
        assert_eq!(decoded.detail, Some("no channel on record".to_owned()));
        assert!(decoded.r#type.ends_with("/not-a-patron"));
    }

    // ── JSON round-trip ───────────────────────────────────────────────────────────

    #[test]
    fn json_round_trip() {
        let original = Error::Unauthorized.with_detail("clock skew was 5s");
        let bytes = original.to_json();
        let decoded: ProblemDetailBody = serde_json::from_slice(&bytes).expect("decode failed");

        assert_eq!(decoded.title, "Unauthorized");
        assert_eq!(decoded.status, 401);
        assert_eq!(decoded.detail, Some("clock skew was 5s".to_owned()));
        assert!(decoded.r#type.ends_with("/unauthorized"));
    }

    #[test]
    fn json_omits_none_fields() {
        let bytes = Error::Unauthorized.to_json();
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(
            v.get("detail").is_none(),
            "detail should be absent when None"
        );
        assert!(
            v.get("instance").is_none(),
            "instance should be absent when None"
        );
    }

    // ── Manifest ──────────────────────────────────────────────────────────────────

    #[test]
    fn manifest_is_valid_json() {
        let manifest: serde_json::Value = serde_json::from_str(Error::PROBLEM_DETAILS_MANIFEST)
            .expect("manifest is not valid JSON");

        let entries = manifest.as_array().expect("manifest should be an array");
        assert_eq!(entries.len(), 2);

        let unauthorized = entries
            .iter()
            .find(|e| e["slug"] == "unauthorized")
            .expect("missing unauthorized entry");

        assert_eq!(unauthorized["title"], "Unauthorized");
        assert_eq!(unauthorized["http_status"], 401);
        assert_eq!(unauthorized["doc"], "MAC verification failed.");
    }
}

#[cfg(test)]
mod tests_part_ii {

    use super::*;

    #[derive(Debug, ProblemDetail)]
    enum CommonError {
        /// Insufficient funds to complete the request.
        #[problem(
            slug = "insufficient-funds",
            title = "Insufficient Funds",
            http_status = 402
        )]
        InsufficientFunds,
    }

    #[derive(Debug, ProblemDetail)]
    enum Error {
        /// MAC verification failed.
        #[problem(slug = "unauthorized", title = "Unauthorized", http_status = 401)]
        Unauthorized,

        /// Keytag is not a recognised patron.
        #[problem(slug = "not-a-patron", title = "Not a Patron", http_status = 403)]
        NotPatron,

        #[problem(delegate)]
        Common(CommonError),
    }

    // ── Derive output ─────────────────────────────────────────────────────────────

    #[test]
    fn slug() {
        assert_eq!(Error::Unauthorized.slug(), "unauthorized");
        assert_eq!(Error::NotPatron.slug(), "not-a-patron");
    }

    #[test]
    fn title() {
        assert_eq!(Error::Unauthorized.title(), "Unauthorized");
        assert_eq!(Error::NotPatron.title(), "Not a Patron");
    }

    #[test]
    fn http_status() {
        assert_eq!(Error::Unauthorized.http_status(), 401);
        assert_eq!(Error::NotPatron.http_status(), 403);
    }

    #[test]
    fn problem_type_uri() {
        // PROBLEM_DETAIL_BASE_URL is set in .cargo/config.toml
        let uri = Error::Unauthorized.problem_type();
        assert!(uri.starts_with("https://konduit.channel/errors/"));
        assert!(uri.ends_with("/unauthorized"));
    }

    // ── WithContext ───────────────────────────────────────────────────────────────

    #[test]
    fn with_context_overrides_detail() {
        let e = Error::Unauthorized.with_detail("clock skew was 42s");
        assert_eq!(e.slug(), "unauthorized");
        assert_eq!(e.http_status(), 401);
        assert_eq!(e.detail(), Some("clock skew was 42s".to_owned()));
    }

    #[test]
    fn with_instance_sets_both() {
        let e = Error::NotPatron.with_instance("keytag abc has no channel", "/requests/99");
        assert_eq!(e.detail(), Some("keytag abc has no channel".to_owned()));
        assert_eq!(e.instance(), Some("/requests/99".to_owned()));
    }

    #[test]
    fn base_error_has_no_detail() {
        assert_eq!(Error::Unauthorized.detail(), None);
    }

    // ── CBOR round-trip ───────────────────────────────────────────────────────────

    #[test]
    fn cbor_round_trip() {
        let original = Error::NotPatron.with_detail("no channel on record");
        let bytes = original.to_cbor();
        let decoded: ProblemDetailBody = minicbor::decode(&bytes).expect("decode failed");

        assert_eq!(decoded.title, "Not a Patron");
        assert_eq!(decoded.status, 403);
        assert_eq!(decoded.detail, Some("no channel on record".to_owned()));
        assert!(decoded.r#type.ends_with("/not-a-patron"));
    }

    // ── JSON round-trip ───────────────────────────────────────────────────────────

    #[test]
    fn json_round_trip() {
        let original = Error::Unauthorized.with_detail("clock skew was 5s");
        let bytes = original.to_json();
        let decoded: ProblemDetailBody = serde_json::from_slice(&bytes).expect("decode failed");

        assert_eq!(decoded.title, "Unauthorized");
        assert_eq!(decoded.status, 401);
        assert_eq!(decoded.detail, Some("clock skew was 5s".to_owned()));
        assert!(decoded.r#type.ends_with("/unauthorized"));
    }

    #[test]
    fn json_omits_none_fields() {
        let bytes = Error::Unauthorized.to_json();
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(
            v.get("detail").is_none(),
            "detail should be absent when None"
        );
        assert!(
            v.get("instance").is_none(),
            "instance should be absent when None"
        );
    }

    // ── Delegate ──────────────────────────────────────────────────────────────────

    #[test]
    fn delegate_slug() {
        assert_eq!(
            Error::Common(CommonError::InsufficientFunds).slug(),
            "insufficient-funds"
        );
    }

    #[test]
    fn delegate_http_status() {
        assert_eq!(
            Error::Common(CommonError::InsufficientFunds).http_status(),
            402
        );
    }

    #[test]
    fn delegate_problem_type_points_at_common_crate() {
        let uri = Error::Common(CommonError::InsufficientFunds).problem_type();
        assert!(uri.ends_with("/insufficient-funds"));
        // type URI contains the crate where CommonError is defined, not Error's crate
        assert!(uri.contains(env!("CARGO_PKG_NAME")));
    }

    #[test]
    fn delegate_cbor_round_trip() {
        let e = Error::Common(CommonError::InsufficientFunds);
        let bytes = e.to_cbor();
        let decoded: ProblemDetailBody = minicbor::decode(&bytes).expect("decode failed");
        assert_eq!(decoded.status, 402);
        assert_eq!(decoded.title, "Insufficient Funds");
    }

    #[test]
    fn delegate_not_in_manifest() {
        let manifest: serde_json::Value =
            serde_json::from_str(Error::PROBLEM_DETAILS_MANIFEST).expect("valid JSON");
        let entries = manifest.as_array().unwrap();
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().all(|e| e["slug"] != "insufficient-funds"));
    }

    #[test]
    fn manifest_is_valid_json() {
        let manifest: serde_json::Value = serde_json::from_str(Error::PROBLEM_DETAILS_MANIFEST)
            .expect("manifest is not valid JSON");

        let entries = manifest.as_array().expect("manifest should be an array");
        assert_eq!(entries.len(), 2);

        let unauthorized = entries
            .iter()
            .find(|e| e["slug"] == "unauthorized")
            .expect("missing unauthorized entry");

        assert_eq!(unauthorized["title"], "Unauthorized");
        assert_eq!(unauthorized["http_status"], 401);
        assert_eq!(unauthorized["doc"], "MAC verification failed.");
    }
}
