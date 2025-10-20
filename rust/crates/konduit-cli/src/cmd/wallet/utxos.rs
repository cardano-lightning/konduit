use crate::{env, metavar};
use anyhow::anyhow;
use cardano_connect::CardanoConnect;
use cardano_tx_builder as cardano;
use cardano_tx_builder::{Credential, Hash, VerificationKey};

/// Fetch UTxO entries at the wallet's address; requires `Cardano` connection
#[derive(Debug, clap::Args)]
#[clap(disable_version_flag(true))]
pub(crate) struct Args {
    /// Wallet's private key; provide either this or --public-key
    #[clap(long, value_name = metavar::ED25519_PRIVATE_KEY, env = env::WALLET_PRIVATE_KEY)]
    private_key: Option<cardano::SigningKey>,

    /// Wallet's public key; provide either this or --private-key
    #[clap(long, value_name = metavar::ED25519_PUBLIC_KEY, env = env::WALLET_PUBLIC_KEY)]
    public_key: Option<cardano::VerificationKey>,
}

impl Args {
    pub(crate) async fn execute(self, connector: impl CardanoConnect) -> anyhow::Result<()> {
        let public_key = self
            .private_key
            .as_ref()
            .map(VerificationKey::from)
            .or(self.public_key)
            .ok_or(anyhow!(
                "missing both --private-key and --public-key; please provide at least one"
            ))?;
        let payment_credential = Credential::from_key(Hash::<28>::new(public_key));
        let utxo = connector.utxos_at(&payment_credential, None).await?;

        // TODO: Use JSON
        for (input, output) in utxo {
            println!("{input}: {output}");
        }

        Ok(())
    }
}
