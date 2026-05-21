/// A 32-byte HMAC key decoded from a hex string.
#[derive(Clone)]
pub struct HmacKey(pub [u8; 32]);

impl std::str::FromStr for HmacKey {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = hex::decode(s).map_err(|e| e.to_string())?;
        let arr: [u8; 32] = bytes
            .try_into()
            .map_err(|_| "HMAC key must be exactly 32 bytes (64 hex chars)".to_string())?;
        Ok(Self(arr))
    }
}

impl std::fmt::Debug for HmacKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HmacKey([redacted])")
    }
}

#[derive(Debug, Clone, clap::Args)]
pub struct ServerArgs {
    #[arg(long, env = crate::env::SERVER_HOST, default_value = "127.0.0.1")]
    pub host: String,
    #[arg(long, env = crate::env::SERVER_PORT, default_value = "5663")]
    pub port: u16,
    /// 32-byte HMAC-BLAKE3 signing key for session tokens, hex-encoded.
    #[arg(long, env = crate::env::HMAC_KEY, hide_env_values = true)]
    pub hmac_key: HmacKey,
}
