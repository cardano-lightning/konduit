//! Command-line metavar specifying the type/semantic of arguments and/or options.

/// A time duration in milliseconds
pub const DURATION: &str = "DURATION";

/// Ed25519 Public key
pub const ED25519_VERIFICATION_KEY: &str = "ED25519_VERIFICATION_KEY";

/// A blake2b_224 script hash
pub const SCRIPT_HASH: &str = "SCRIPT_HASH<28>";

/// A transaction output reference in the form <tx-hash>#<index>
pub const TXOUT_REF: &str = "TXOUT_REF";
