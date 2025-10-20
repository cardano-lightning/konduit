use crate::{env, metavar};
use anyhow::anyhow;
use cardano_connect::CardanoConnect;
use cardano_tx_builder::{Credential, Hash, SigningKey, VerificationKey};

/// Fetch UTxO entries at the wallet's address; requires `Cardano` connection
#[derive(Debug, clap::Args)]
#[clap(disable_version_flag(true))]
pub(crate) struct Args {
    /// Wallet's signing key; provide either this or --verification-key
    #[clap(
        long,
        value_name = metavar::ED25519_SIGNING_KEY,
        env = env::WALLET_SIGNING_KEY
    )]
    signing_key: Option<SigningKey>,

    /// Wallet's verification key; provide either this or --signing-key
    #[clap(
        long,
        value_name = metavar::ED25519_VERIFICATION_KEY,
        env = env::WALLET_VERIFICATION_KEY,
    )]
    verification_key: Option<VerificationKey>,
}

impl Args {
    pub(crate) async fn execute(self, connector: impl CardanoConnect) -> anyhow::Result<()> {
        let verification_key = self
            .signing_key
            .as_ref()
            .map(VerificationKey::from)
            .or(self.verification_key)
            .ok_or(anyhow!(
                "missing both --signing-key and --verification-key; please provide at least one"
            ))?;

        let payment_credential = Credential::from_key(Hash::<28>::new(verification_key));

        let utxo = connector.utxos_at(&payment_credential, None).await?;

        // TODO: Use JSON
        for (input, output) in utxo {
            println!("{input}: {output}");
        }

        Ok(())
    }
}
