use crate::{env, metavar};
use anyhow::anyhow;
use cardano_tx_builder::{Address, Credential, Hash, NetworkId, SigningKey, VerificationKey};
use serde_json::json;

/// Display useful pieces of informations about a known wallet
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

    /// Whether to show data from a testnet perspective
    #[clap(
        long("testnet"),
        default_value_t = NetworkId::MAINNET,
        num_args = 0..=1,
        default_missing_value = "testnet",
        hide_default_value = true,
    )]
    network_id: NetworkId,
}

impl Args {
    pub(crate) fn execute(self) -> anyhow::Result<()> {
        let verification_key = self
            .signing_key
            .as_ref()
            .map(VerificationKey::from)
            .or(self.verification_key)
            .ok_or(anyhow!(
                "missing both --signing-key and --verification-key; please provide at least one"
            ))?;

        let payment_credential = Credential::from_key(Hash::<28>::new(verification_key));

        let address = Address::new(self.network_id, payment_credential);

        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "address": address.to_string(),
                "verification_key": verification_key.to_string(),
            }))
            .unwrap()
        );

        Ok(())
    }
}
