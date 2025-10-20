use crate::{env, metavar};
use cardano_tx_builder as cardano;
use cardano_tx_builder::{Address, Credential, Hash, NetworkId, VerificationKey};
use serde_json::json;

/// Display useful pieces of informations about a known wallet
#[derive(Debug, clap::Args)]
#[clap(disable_version_flag(true))]
pub(crate) struct Args {
    /// Wallet's signing key
    #[clap(long, value_name = metavar::ED25519_PRIVATE_KEY, env = env::WALLET_PRIVATE_KEY)]
    private_key: cardano::SigningKey,

    /// Whether to show data from a testnet perspective
    #[clap(long, default_value = "true")]
    testnet: bool,
}

impl Args {
    pub(crate) fn execute(self) -> anyhow::Result<()> {
        // TODO: Move NetworkId to 'Args', ensuring it still behave like a flag.
        let network_id = if self.testnet {
            NetworkId::TESTNET
        } else {
            NetworkId::MAINNET
        };
        let public_key = VerificationKey::from(&self.private_key);
        let payment_credential = Credential::from_key(Hash::<28>::new(public_key));
        let address = Address::new(network_id, payment_credential);

        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "address": address.to_string(),
                "public_key": public_key.to_string(),
            }))
            .unwrap()
        );

        Ok(())
    }
}
