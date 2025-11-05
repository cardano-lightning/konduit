//! Environment variable names used throughout the command-line. Certain flags/options can be
//! passed directly as environment variables, which allows caller to maintain a somewhat consistent
//! state.

// Blockfrost project id
pub const BLOCKFROST_PROJECT_ID: &str = "KONDUIT_BLOCKFROST_PROJECT_ID";

/// Wallet's Ed25519 Private Key
pub const WALLET_SIGNING_KEY: &str = "KONDUIT_WALLET_SIGNING_KEY";

/// Wallet's Ed25519 Public Key
pub const WALLET_VERIFICATION_KEY: &str = "KONDUIT_WALLET_VERIFICATION_KEY";

/// Consumer's Ed25519 Public Key
pub const CONSUMER: &str = "KONDUIT_CONSUMER";

/// Adaptor's Ed25519 Public Key
pub const ADAPTOR: &str = "KONDUIT_ADAPTOR";

/// Channel (somewhat) unique tag
pub const CHANNEL_TAG: &str = "KONDUIT_CHANNEL_TAG";

/// Minimum time between the 'close' and 'elapse' states.
pub const CLOSE_PERIOD: &str = "KONDUIT_CLOSE_PERIOD";

/// Script hash of the Konduit validator if other than the default embedded one.
pub const SCRIPT_HASH: &str = "KONDUIT_SCRIPT_HASH";
