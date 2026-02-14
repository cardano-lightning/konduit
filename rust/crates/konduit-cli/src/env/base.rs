use crate::config::signing_key::SigningKey;
use cardano_tx_builder::{Address, Credential, Hash, NetworkId, VerificationKey, address::kind};
use std::fs;

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

pub fn load_if_exists(path: &str) -> anyhow::Result<()> {
    if fs::exists(path)? {
        dotenvy::from_filename(path).map_err(|err| anyhow::anyhow!("{}", err))?;
    }
    Ok(())
}
