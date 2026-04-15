use cardano_sdk::cbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

/// Version should work for _all_ clients with _all_ servers.
/// Thus `Response` datatype should *never* change.
/// All other endpoints may be entirely different, but
/// at least a client will be able to establish incompatibility from the version.
#[derive(Serialize, Deserialize, Encode, Decode)]
pub struct Response {
    /// Support diversity in versioning.
    /// If you want to make a new version, make the flavour distinct and identifiable.
    /// The default flavour is `default`
    #[n(0)]
    pub flavor: String,

    /// Bumped on any breaking change. Client must match exactly.
    #[n(1)]
    pub sem_ver: crate::core::SemVer,

    /// What encodings this server accepts.
    #[n(2)]
    pub formats: Vec<crate::core::MediaFormat>,

    /// Content hash of protocol-relevant crates.
    /// Stable across monorepo changes, changes when wire behavior can change.
    #[n(3)]
    pub protocol_hash: String,

    /// Git hash for cross-referencing with VCS. Informational only —
    /// may be "unknown" in Nix or hermetic builds.
    #[n(4)]
    pub git_hash: String,
}
