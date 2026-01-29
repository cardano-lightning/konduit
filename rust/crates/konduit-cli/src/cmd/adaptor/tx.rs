use std::collections::BTreeMap;

use konduit_data::{Keytag, Receipt};
use tokio::runtime::Runtime;

use cardano_connect::CardanoConnect;
use cardano_tx_builder::Credential;

use konduit_tx::{self, Bounds, KONDUIT_VALIDATOR, NetworkParameters, adaptor::AdaptorPreferences};

use crate::{cmd::parsers::parse_keytag_receipt, config::adaptor::Config};

/// Create and submit Konduit transactions
#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    /// Receipts are semicolon separated list.
    /// The first two items must be keytag and squash respectively;
    /// the rest are cheques.
    /// There are a few accepted formats of squash and of cheques
    /// Format : <keytag>;<squash>;<cheque_0>;<cheque_1> ...
    /// squash_body,signature;cheque_body,signature,secret;cheque,secret;
    #[arg(long, value_parser=parse_keytag_receipt)]
    pub receipt: Vec<(Keytag, Receipt)>,
}

impl Cmd {
    pub fn run(self, config: &Config) -> anyhow::Result<()> {
        let connector = config.connector.connector()?;
        let own_key = config.wallet.to_verification_key();
        let own_address = config.wallet.to_address(&connector.network().into());
        let receipts = self.receipt.into_iter().collect::<BTreeMap<_, _>>();
        let preferences = AdaptorPreferences {
            min_single: 10_000,
            min_total: 1_000_000,
        };
        let bounds = Bounds::twenty_mins();

        Runtime::new()?.block_on(async {
            let protocol_parameters = connector.protocol_parameters().await?;
            let network_id = connector.network().into();
            let network_parameters = NetworkParameters {
                network_id,
                protocol_parameters,
            };
            let utxos = connector
                .utxos_at(&own_address.payment(), None)
                .await?
                .into_iter()
                .chain(
                    connector
                        .utxos_at(
                            &config.host_address.payment(),
                            config.host_address.delegation().as_ref(),
                        )
                        .await?
                        .into_iter(),
                )
                .chain(
                    connector
                        .utxos_at(&Credential::from_script(KONDUIT_VALIDATOR.hash), None)
                        .await?
                        .into_iter(),
                )
                .collect();
            let mut tx = konduit_tx::adaptor::tx(
                &network_parameters,
                &preferences,
                &own_key,
                &receipts,
                &utxos,
                &bounds.upper,
            )?;
            println!("Tx id :: {}", tx.id());
            tx.sign(config.wallet.clone().into());
            connector.submit(&tx).await
        })
    }
}
