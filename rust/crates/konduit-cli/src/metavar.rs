//! Command-line metavar specifying the type/semantic of arguments and/or options.

/// Lovelace amounts
pub const LOVELACE: &str = "LOVELACE";

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

/// A CBOR file which contains Plutus data
pub const PLUTUS_CBOR_FILE: &str = "PLUTUS_DATA_CBOR_FILE";

/// A transaction output reference in the form <tx-hash>#<index>
pub const OUTPUT_REF: &str = "TRANSACTION_ID#OUTPUT_INDEX";
