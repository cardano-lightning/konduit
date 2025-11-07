use crate::{env, metavar};
use cardano_connect::CardanoConnect;
use cardano_tx_builder::{Address, Credential, Hash, SigningKey, VerificationKey, address};
use konduit_tx::{KONDUIT_VALIDATOR, deploy};

#[derive(Debug, clap::Args)]
#[clap(disable_version_flag(true))]
pub(crate) struct Args {
    // The publisher wallet's signing key
    #[clap(
        long,
        value_name = metavar::ED25519_SIGNING_KEY,
        env = env::WALLET_SIGNING_KEY
    )]
    signing_key: SigningKey,

    #[arg(short = 'd', long)]
    dry_run: bool,
}

impl Args {
    pub(crate) async fn execute(self, connector: impl CardanoConnect) -> anyhow::Result<()> {
        let verification_key = VerificationKey::from(&self.signing_key);
        let payment_credential = Credential::from_key(Hash::<28>::new(verification_key));
        let utxo = connector.utxos_at(&payment_credential, None).await?;
        let protocol_parameters = connector.protocol_parameters().await?;
        let konduit_script = KONDUIT_VALIDATOR.script.clone();
        let network = connector.network();
        let addr: Address<address::kind::Any> =
            Address::new(network.into(), payment_credential.clone()).into();

        let mut deploy_transaction = deploy(
            &protocol_parameters,
            &utxo,
            konduit_script,
            addr.clone(),
            addr.clone(),
        )?;
        deploy_transaction.sign(self.signing_key);
        if !self.dry_run {
            let tx_id = connector.submit(&deploy_transaction).await?;
            println!("Transaction submitted with ID: {}", tx_id);
        }

        println!("{deploy_transaction}");
        Ok(())
    }
}
