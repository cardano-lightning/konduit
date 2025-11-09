use crate::cmd::metavar;
use cardano_tx_builder::{Hash, VerificationKey};
use clap::Args;
use konduit_data::Duration;

fn parse_hex<const LEN: usize>(s: &str) -> Result<[u8; LEN], String> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    let bytes = hex::decode(s).map_err(|e| e.to_string())?;
    <[u8; LEN]>::try_from(bytes).map_err(|_| "Invalid length".to_string())
}

fn parse_script_hash(s: &str) -> Result<Hash<28>, String> {
    let arr: [u8; 28] = parse_hex(s)?;
    Ok(Hash::from(arr))
}

#[derive(Debug, Clone, Args)]
pub struct Info {
    // Amount in channel currency (eg lovelace)
    #[arg(long, env = crate::env::FEE, default_value = "1000")]
    pub fee: u64,
    #[arg(long, env = crate::env::ADAPTOR_VKEY)]
    pub adaptor_key: VerificationKey,
    #[arg(long, env = crate::env::CLOSE_PERIOD, value_name=metavar::DURATION, default_value="24h")]
    pub close_period: Duration,
    #[arg(long, env = crate::env::DEPLOYER_VKEY, value_name=metavar::ED25519_VERIFICATION_KEY)]
    pub deployer_vkey: VerificationKey,
    #[arg(long, env = crate::env::SCRIPT_HASH, value_name=metavar::SCRIPT_HASH, value_parser = parse_script_hash)]
    pub script_hash: Hash<28>,
    #[arg(long, env = crate::env::MAX_TAG_LENGTH, default_value = "32")]
    pub max_tag_length: usize,
}
