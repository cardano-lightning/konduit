use crate::{env, metavar};
use anyhow::anyhow;
use cardano_connect::CardanoConnect;
use cardano_tx_builder::{Credential, Datum, Hash, SigningKey, VerificationKey};
use konduit_data;

#[derive(Debug, Clone, clap::Subcommand)]
pub(crate) enum Role {
    Adaptor,
    Consumer(ConsumerArgs),
}

#[derive(Debug, Clone, clap::Args)]
pub(crate) struct ConsumerArgs {
    #[clap(
        long,
        long_help = "By default we use consumer verification key to filter channel UTxOs which are delegated to that staking key."
    )]
    ignore_staking_key: bool,
}

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

    #[clap(long, value_name = metavar::SCRIPT_HASH, env = env::SCRIPT_HASH, default_value_t = crate::KONDUIT_VALIDATOR.hash)]
    konduit_script_hash: Hash<28>,

    #[clap(long, value_name = metavar::BYTES_32, env = env::CHANNEL_TAG)]
    channel_tag: Option<konduit_data::Tag>,

    #[clap(subcommand)]
    role: Role,
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
        let script_credential = Credential::from_script(self.konduit_script_hash);
        // let staking_credential: Option<Credential> = match &self.role {
        //     Role::Consumer(args) => {
        //         if !args.ignore_staking_key {
        //             Some(Credential::from_key(Hash::<28>::new(
        //                 verification_key.clone(),
        //             )))
        //         } else {
        //             None
        //         }
        //     }
        //     Role::Adaptor => None,
        // };

        let utxos = connector
            .utxos_at(&script_credential, None) // , staking_credential.as_ref())
            .await?;

        let channels = utxos
            .into_iter()
            .filter_map(|(input, output)| {
                let datum_rc: &Datum = output.datum()?;
                match datum_rc {
                    Datum::Inline(plutus_data) => {
                        let konduit_datum = konduit_data::Datum::try_from(plutus_data).ok()?;
                        Some((input, konduit_datum))
                    }
                    Datum::Hash(_) => None,
                }
            })
            .filter(|(_, datum)| {
                println!("Found channel datum with tag: {:?}", datum.constants.tag);
                println!("Looking for channel: {:?}", self.channel_tag);
                println!("Datum: {:?}", datum);
                if let Some(tag) = &self.channel_tag {
                    &datum.constants.tag == tag
                } else {
                    true
                }
            })
            .collect::<Vec<_>>();

        // TODO: Provide a way to use JSON as an output format
        for (input, datum) in channels {
            println!("Channel UTxO: {}", input);
            println!("Datum: {datum:#?}");
            println!();
        }

        Ok(())
    }
}
