use std::collections::HashMap;

use cardano_tx_builder::{Address, Credential, Hash, NetworkId, VerificationKey, address::kind};

use crate::config::signing_key::SigningKey;

pub const PREFIX: &str = "KONDUIT_";

pub fn placeholder_address(network_id: Option<NetworkId>) -> Address<kind::Shelley> {
    Address::new(
        network_id.unwrap_or(NetworkId::MAINNET),
        Credential::from_key(Hash::<28>::new(vec![])),
    )
}

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
