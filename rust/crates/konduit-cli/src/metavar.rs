//! Command-line metavar specifying the type/semantic of arguments and/or options.

/// Lovelace amounts
pub const LOVELACE: &str = "U64";

/// Ed25519 Public key
pub const ED25519_VERIFICATION_KEY: &str = "ED25519_PUB";

/// Ed25519 Private key
pub const ED25519_SIGNING_KEY: &str = "ED25519_PRV";

/// 32-bytes Tag
pub const BYTES_32: &str = "HEX32";

/// A time duration
pub const DURATION: &str = "DURATION";

/// A script hash
pub const SCRIPT_HASH: &str = "SCRIPT_HASH";
