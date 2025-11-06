use crate::{env, metavar};
use cardano_connect::CardanoConnect;
use cardano_tx_builder::{
    self as cardano, Hash, Input, Output, PlutusData, SigningKey, VerificationKey, cbor,
};
use konduit_data::{self as konduit};
use konduit_tx::sub;

#[derive(Debug, clap::Args)]
#[clap(disable_version_flag(true))]
pub(crate) struct Args {
    // We also assume that the consumer is opening that channel and paying for it.
    #[clap(
        long,
        value_name = metavar::ED25519_SIGNING_KEY,
        env = env::WALLET_SIGNING_KEY
    )]
    adaptor_signing_key: SigningKey,

    // If ommited then we assume that the reference script is published at the adaptor's address
    #[clap(long, value_name = metavar::ED25519_VERIFICATION_KEY)]
    published_by: Option<VerificationKey>,

    // An (ideally) unique tag to discriminate channels and allow reuse of keys between them.
    #[clap(long, value_name = metavar::BYTES_32, env = env::CHANNEL_TAG)]
    channel_tag: konduit::Tag,

    // Adaptor's verification key, allowed to *sub* funds
    #[clap(long, value_name = metavar::ED25519_VERIFICATION_KEY, env = env::ADAPTOR)]
    adaptor_verification_key: cardano::VerificationKey,

    #[arg(short = 'd', long)]
    dry_run: bool,

    #[clap(
        long,
        value_name = metavar::PLUTUS_CBOR_FILE,
    )]
    squash_file: String,
}

fn output_reference_script_hash(output: &Output) -> Option<Hash<28>> {
    output
        .script()
        .map(|script| cardano::Hash::<28>::from(script))
}

impl Args {
    pub(crate) async fn execute(self, connector: impl CardanoConnect) -> anyhow::Result<()> {
        let adaptor_verification_key = VerificationKey::from(&self.adaptor_signing_key);
        let adaptor_payment_credential = cardano::Credential::from_key(cardano::Hash::<28>::new(
            adaptor_verification_key.clone(),
        ));

        let publisher_credential = {
            let vk = match self.published_by {
                Some(vk) => vk,
                None => adaptor_verification_key,
            };
            cardano::Credential::from_key(cardano::Hash::<28>::new(vk))
        };
        println!("Publisher credential: {}", publisher_credential);
        println!("Adaptor credential: {}", adaptor_payment_credential);

        let funding_utxos = connector
            .utxos_at(&adaptor_payment_credential, None)
            .await?;

        let protocol_parameters = connector.protocol_parameters().await?;
        let network_id = cardano::NetworkId::from(connector.network());
        let script_utxo = {
            let publisher_utxos = connector.utxos_at(&publisher_credential, None).await?;
            publisher_utxos
                .into_iter()
                .find(|(_input, output)| {
                    output_reference_script_hash(output) == Some(crate::KONDUIT_VALIDATOR.hash)
                })
                .ok_or_else(|| anyhow::anyhow!("could not find konduit script UTXO"))?
        };

        let channel_utxos = connector
            .utxos_at(
                &cardano::Credential::from_script(crate::KONDUIT_VALIDATOR.hash),
                None,
            )
            .await?;
        let channel_utxos = channel_utxos.into_iter().filter(|(_, output)| {
            if let Some(datum) = output.datum() {
                match datum {
                    cardano::Datum::Inline(plutus_data) => {
                        konduit_data::Datum::try_from(plutus_data)
                            .ok()
                            .map(|konduit_datum| {
                                println!("Found channel datum with tag: {:?}", konduit_datum);
                                konduit_datum.constants.tag == self.channel_tag
                                    && konduit_datum.constants.sub_vkey == adaptor_verification_key
                            })
                            .unwrap_or(false)
                    }
                    cardano::Datum::Hash(_) => false,
                }
            } else {
                false
            }
        });

        let channel_utxos: Vec<(Input, Output)> = channel_utxos.into_iter().collect();
        println!("Found {} channel UTXOs", channel_utxos.len());
        let squash = {
            let squash_cbor = std::fs::read(&self.squash_file)?;
            let plutus_data = cbor::decode::<PlutusData>(&squash_cbor)?;
            konduit::Squash::try_from(plutus_data)?
        };

        let receipt = {
            let unlockeds = vec![];
            konduit_data::Receipt {
                squash: squash,
                unlockeds: unlockeds,
            }
        };

        let mut sub_transaction = sub(
            &funding_utxos,
            &script_utxo,
            &channel_utxos,
            &receipt,
            adaptor_verification_key,
            &protocol_parameters,
            network_id,
        )?;
        match sub_transaction {
            Some(ref mut tx) => {
                tx.sign(self.adaptor_signing_key);
                if !self.dry_run {
                    let tx_id = connector.submit(&tx).await?;
                    println!("Transaction submitted with ID: {}", tx_id);
                } else {
                    println!("Dry run, transaction not submitted.");
                }
            }
            None => {
                println!("No subtraction necessary, transaction not created.");
                return Ok(());
            }
        }

        // println!("{sub_transaction}");

        Ok(())
    }
}
