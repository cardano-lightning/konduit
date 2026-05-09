use cardano_sdk::cbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::api::media_format::MediaFormat;

#[derive(Serialize, Deserialize, Encode, Decode)]
pub struct Version {
    /// Support diversity in versioning.
    /// If you want to make a new version, make the flavour distinct and identifiable.
    /// The default flavour is `default`
    #[n(0)]
    pub flavor: String,

    /// Bumped on any breaking change. Client must match exactly.
    #[n(1)]
    pub sem_ver: SemVer,

    /// What encodings this server accepts.
    #[n(2)]
    pub formats: Vec<MediaFormat>,

    /// Content hash of protocol-relevant crates.
    /// Stable across monorepo changes, changes exactly when wire behavior can change.
    #[n(3)]
    pub protocol_hash: String,

    /// Git hash for cross-referencing with VCS. Informational only —
    /// may be "unknown" in Nix or hermetic builds.
    #[n(4)]
    pub git_hash: String,
}

#[derive(Serialize, Deserialize, Encode, Decode, PartialEq, Eq, Hash, Clone)]
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
