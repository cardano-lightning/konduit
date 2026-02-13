use std::{collections::HashMap, fs};

use cardano_tx_builder::{Address, Credential, Hash, NetworkId, VerificationKey, address::kind};

use crate::config::signing_key::SigningKey;

pub const PREFIX: &str = "KONDUIT_";

pub fn signing_key_to_address(
    network_id: &NetworkId,
    wallet: &SigningKey,
) -> Address<kind::Shelley> {
    Address::new(
        network_id.clone(),
        Credential::from_key(Hash::<28>::new(&VerificationKey::from(
            &<cardano_tx_builder::SigningKey>::from(wallet.clone()),
        ))),
    )
}

pub fn load<T: serde::de::DeserializeOwned>() -> anyhow::Result<T> {
    let mut map = HashMap::new();
    for (key, value) in std::env::vars() {
        if key.starts_with(PREFIX) {
            map.insert(key, value);
        }
    }
    let json = serde_json::to_value(map).expect("Failed to map env vars");
    let x = serde_json::from_value(json)?;
    Ok(x)
}

pub fn load_dotenv(default_path: &str) -> anyhow::Result<()> {
    if fs::exists(default_path)? {
        dotenvy::from_filename(default_path).map_err(|err| anyhow::anyhow!("{}", err))?;
    }
    if fs::exists(".env")? {
        dotenvy::from_filename(".env").map_err(|err| anyhow::anyhow!("{}", err))?;
    }
    Ok(())
}
