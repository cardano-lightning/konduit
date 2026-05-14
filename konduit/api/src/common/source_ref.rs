//! First-class cross-referencing from wire types to their source and specification.
//!
//! When a developer sees an error or event in the wild (logs, bug report, on-call alert),
//! they can recover the exact definition and spec using:
//!
//! ```text
//! # Source code (requires VCS hash from GET /version):
//! git show <vcs_hash>:<E::source_path()>
//!
//! # Hosted documentation (if docs_base_url is set in GET /version):
//! <docs_base_url>/spec/<feature>
//! ```
//!
//! All endpoint error types implement `SourceRef`.
//! The [`version::Response`][crate::endpoints::version::Response] carries the `vcs_hash`
//! and `docs_base_url` needed to resolve these paths at runtime.

/// Implemented by all protocol types to enable cross-referencing with VCS and documentation.
///
/// # Example
///
/// ```ignore
/// use konduit_api::common::source_ref::SourceRef;
/// use konduit_api::endpoints::channel::sync;
///
/// // From a version response you have vcs_hash: "abc1234"
/// let src = sync::Error::source_path();
/// println!("git show abc1234:{src}");
/// ```
pub trait SourceRef {
    /// Path to this type's definition relative to VCS root.
    ///
    /// Combine with `version::Response::vcs_hash` to reproduce the exact wire contract:
    /// `git show <vcs_hash>:<source_path()>`
    fn source_path() -> &'static str;

    /// Path to the human-readable spec for this endpoint, relative to VCS root.
    /// Follows the convention `docs/spec/<feature>.md`.
    ///
    /// Combine with `version::Response::docs_base_url` to get the web URL.
    fn doc_path() -> Option<&'static str> {
        None
    }
}

// ---------------------------------------------------------------------------
// Implementations
// ---------------------------------------------------------------------------

impl SourceRef for crate::auth::pop::Error {
    fn source_path() -> &'static str {
        "crates/konduit-api/src/auth/pop/error.rs"
    }
}

impl SourceRef for crate::endpoints::channel::sync::Error {
    fn source_path() -> &'static str {
        "crates/konduit-api/src/endpoints/channel/sync.rs"
    }
    fn doc_path() -> Option<&'static str> {
        Some("docs/spec/channel.md")
    }
}

impl SourceRef for crate::endpoints::channel::squash::Error {
    fn source_path() -> &'static str {
        "crates/konduit-api/src/endpoints/channel/squash.rs"
    }
    fn doc_path() -> Option<&'static str> {
        Some("docs/spec/channel.md")
    }
}

impl SourceRef for crate::endpoints::channel::pay::quoted::Error {
    fn source_path() -> &'static str {
        "crates/konduit-api/src/endpoints/channel/pay/quoted.rs"
    }
    fn doc_path() -> Option<&'static str> {
        Some("docs/spec/channel.md")
    }
}

impl SourceRef for crate::common::channel::quote::Error {
    fn source_path() -> &'static str {
        "crates/konduit-api/src/common/channel/quote/error.rs"
    }
    fn doc_path() -> Option<&'static str> {
        Some("docs/spec/channel.md")
    }
}
