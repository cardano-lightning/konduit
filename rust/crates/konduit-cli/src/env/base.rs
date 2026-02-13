use crate::{
    config::signing_key::SigningKey,
    shared::{DefaultPath, Fill},
};
use cardano_tx_builder::{Address, Credential, Hash, NetworkId, VerificationKey, address::kind};
use std::{fs, io::IsTerminal};
use toml;

#[derive(Debug, Clone, clap::Args)]
pub struct Setup<E: clap::Args> {
    #[command(flatten)]
    pub env: E,
}

impl<E: clap::Args + DefaultPath + Fill + serde::Serialize> Setup<E> {
    pub fn run(self, env: E) -> anyhow::Result<()> {
        if std::io::stdout().is_terminal() {
            println!("./{}", E::DEFAULT_PATH);
        }
        println!(
            "{:#}",
            toml::to_string(&self.env.fill(env))?.replace(" = ", "=")
        );
        Ok(())
    }
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

pub fn load_dotenv(default_path: &str) -> anyhow::Result<()> {
    if fs::exists(default_path)? {
        dotenvy::from_filename(default_path).map_err(|err| anyhow::anyhow!("{}", err))?;
    }
    if fs::exists(".env")? {
        dotenvy::from_filename(".env").map_err(|err| anyhow::anyhow!("{}", err))?;
    }
    Ok(())
}
