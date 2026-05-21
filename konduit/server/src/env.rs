// Declaration of variable names.

/// # Adaptor wallet
pub const SIGNING_KEY: &str = "KONDUIT_SIGNING_KEY";

/// # Cardano connect
pub const CARDANO_BACKEND: &str = "KONDUIT_CARDANO_BACKEND";
pub const BLOCKFROST_PROJECT_ID: &str = "KONDUIT_BLOCKFROST_PROJECT_ID";
pub const UTXORPC_URI: &str = "KONDUIT_UTXORPC_URI";
pub const NETWORK: &str = "KONDUIT_NETWORK";

/// # Db config
pub const DB_PATH: &str = "KONDUIT_DB_PATH";

/// # Server config
pub const SERVER_HOST: &str = "KONDUIT_SERVER_HOST";
pub const SERVER_PORT: &str = "KONDUIT_SERVER_PORT";
/// 32-byte HMAC key, hex-encoded (64 chars).  Used to sign session tokens.
pub const HMAC_KEY: &str = "KONDUIT_HMAC_KEY";

/// # Channel params
pub const CLOSE_PERIOD: &str = "KONDUIT_CLOSE_PERIOD";
pub const FEE: &str = "KONDUIT_FEE";
pub const TAG_LENGTH: &str = "KONDUIT_TAG_LENGTH";

/// # Tx building & preferences
pub const MIN_SINGLE: &str = "KONDUIT_MIN_SINGLE";
pub const MIN_TOTAL: &str = "KONDUIT_MIN_TOTAL";
/// Host address is the cardano address hosting the konduit validator reference script.
pub const HOST_ADDRESS: &str = "KONDUIT_HOST_ADDRESS";

/// # BLN
pub const BLN_MACAROON: &str = "KONDUIT_BLN_MACAROON";
pub const BLN_TLS: &str = "KONDUIT_BLN_TLS";
pub const BLN_URL: &str = "KONDUIT_BLN_URL";

/// # Fx
pub const COIN_GEKO_TOKEN: &str = "KONDUIT_COIN_GECKO_TOKEN";
pub const FX_ADA: &str = "KONDUIT_FX_ADA";
pub const FX_BITCOIN: &str = "KONDUIT_FX_BITCOIN";
