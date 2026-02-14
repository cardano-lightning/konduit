use cardano_tx_builder::{Address, LeakableSigningKey, NetworkId, SigningKey, address::kind};
use std::fs;

pub fn default_wallet_and_address(
    network_id: NetworkId,
    wallet: Option<LeakableSigningKey>,
    host_address: Option<Address<kind::Shelley>>,
) -> (LeakableSigningKey, Address<kind::Shelley>) {
    let wallet = wallet.unwrap_or(SigningKey::new().into());
    let host_address = host_address.unwrap_or(wallet.to_verification_key().to_address(network_id));
    (wallet, host_address)
}

pub fn load_if_exists(path: &str) -> anyhow::Result<()> {
    if fs::exists(path)? {
        dotenvy::from_filename(path).map_err(|err| anyhow::anyhow!("{}", err))?;
    }
    Ok(())
}
