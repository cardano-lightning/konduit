use std::collections::BTreeMap;

use cardano_sdk::cbor::{Decode, Encode};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Version should work for _all_ clients with _all_ servers.
/// Thus `Response` datatype should *never* change.
/// All other endpoints may be entirely different, but
/// at least a client will be able to establish incompatibility from the version.
#[derive(Debug, Clone, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Response {
    /// Support diversity in versioning.
    /// If you want to make a new version, make the flavour distinct and identifiable.
    /// The default flavour is `default`
    #[n(0)]
    pub flavor: String,

    /// Bumped on any breaking change.
    /// At clients discression how to proceed.
    /// This provides a friendly way to navigate to the correct docs.
    /// It is much more flexible than `protocol_hash`
    #[n(1)]
    pub release: SemVer,

    /// Content hash of protocol-relevant crates.
    /// Stable across monorepo changes: changes only when wire behavior can change.
    /// More refined than git hash, but also much more complicated.
    #[n(2)]
    pub protocol_hash: String,

    /// Git hash for cross-referencing with VCS.
    /// Simpler than `protocol_hash` and more useful when needing to checkout the source code.
    /// However it is also noisier: irrelevant changes will be picked up.
    #[n(3)]
    pub vcs_hash: VcsHash,

    /// What features are supported.
    #[n(4)]
    pub features: BTreeMap<String, FeatureInfo>,

    /// Base URL for hosted documentation for this exact release.
    /// Constructed as `<host>/docs/<vcs_hash>` at build time.
    /// Append `doc_path` (with "docs/" stripped and ".md" removed) to get the web URL.
    /// Example: "https://myorg.github.io/myrepo/abc1234f"
    ///
    /// Self-hosters set this to their own docs server.
    /// In Unknown/Dirty builds this may be empty — use git or local access instead.
    #[n(5)]
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub docs_base_url: Option<String>,
}

#[derive(Debug, Clone, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FeatureInfo {
    /// Bumped on any breaking change.
    /// At clients discression how to proceed.
    /// Versioning at the feature level allows for
    /// extensions to be worked on independently
    #[n(0)]
    pub version: SemVer,

    /// Truncated SHA-256 (8 bytes) over the canonical schema definition.
    /// Fast mismatch detection across independently-versioned features.
    /// Not collision-resistant — use `version` for authoritative compatibility checks.
    #[n(1)]
    #[cfg_attr(
        feature = "serde",
        serde(with = "serde_with::As::<serde_with::hex::Hex>")
    )]
    pub schema_hash: [u8; 8],

    /// Path to the human-readable spec, relative to VCS root.
    /// Always of the form "docs/spec/<feature>.md".
    ///
    /// Three access methods, all mechanically derived from this path:
    ///    git:   `git show <vcs_hash>:doc_path`
    ///    local: `docs/book/spec/<feature>.html`  (after `mdbook build`)
    ///    web:   `<Response::docs_base_url>/spec/<feature>`
    ///
    /// The convention "docs/spec/*.md" → "/spec/*" must be preserved
    /// for all three to stay in correspondence.
    #[n(2)]
    pub doc_path: String,

    /// Path to the canonical type definition in the *protocol* crate, relative to VCS root.
    /// This is the wire contract — not the downstream implementation.
    /// `git show <vcs_hash>:source_path`
    /// Example: "crates/konduit-api/src/endpoints/channel/sync.rs"
    #[n(3)]
    pub source_path: String,
}

#[derive(Debug, Clone, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum VcsHash {
    /// Standard & Prod builds _ought_ to advertise their VCS hash.
    #[n(0)]
    Known(#[n(0)] String),
    /// Local modifications on top of a known base commit.
    /// The inner string is the base commit SHA — working tree may diverge.
    /// Expected in development environments; should not appear in releases.
    #[n(1)]
    Dirty(#[n(0)] String),
    /// VCS information unavailable — hermetic or reproducible builds (e.g. Nix)
    /// where injecting git state would break reproducibility.
    /// Also acceptable in sandboxed CI environments.
    #[n(2)]
    Unknown,
}

#[derive(Debug, Encode, Decode, PartialEq, Eq, Hash, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SemVer {
    /// Bumped on any breaking change
    #[n(0)]
    pub major: u16,
    /// Bumped on additive change
    #[n(1)]
    pub minor: u16,
    /// Bumped on bug fix/ patch
    #[n(2)]
    pub patch: u16,
}
