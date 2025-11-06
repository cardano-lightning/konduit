use clap::Args;
use serde::{Deserialize, Serialize};

const KEY_LEN: usize = 32;

fn parse_hex_key(s: &str) -> Result<[u8; KEY_LEN], String> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    if s.len() != KEY_LEN * 2 {
        return Err(format!("Expected {} hex chars", KEY_LEN * 2));
    }
    let vec = hex::decode(s).map_err(|e| e.to_string())?;
    vec.try_into()
        .map_err(|_| "Internal length error".to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize, Args)]
pub struct Info {
    #[arg(long, env = "KONDUIT_INFO_FEE", default_value = "1000")]
    pub fee: u64,
    #[arg(long, env = "KONDUIT_INFO_ADAPTOR", value_parser = parse_hex_key)]
    #[serde(with = "hex")]
    pub adaptor_key: [u8; 32],
    #[arg(long, env = "KONDUIT_INFO_CLOSE_PERIOD", default_value = "86400000")]
    pub close_period: u64,
}
