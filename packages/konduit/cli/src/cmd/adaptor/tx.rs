use crate::output::{self, OutputJson};
use crate::{cmd::parsers::parse_keytag_receipt, config::adaptor::Config};
use cardano_connector::{CardanoConnector, chain_now};
use cardano_sdk::{Credential, Slot};
use konduit_tmp::{Keytag, Receipt};
use konduit_tx::{self, Bounds, KONDUIT_VALIDATOR, NetworkParameters, adaptor::AdaptorPreferences};
use std::collections::BTreeMap;

/// Create and submit Konduit transactions
#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    /// Receipts are semicolon separated list.
    /// The first two items must be keytag and squash respectively;
    /// the rest are cheques.
    /// There are a few accepted formats of squash and of cheques
    /// Format : `keytag;squash;cheque_0;cheque_1;...`
    /// squash_body,signature;cheque_body,signature,secret;cheque,secret;
    #[arg(long, value_parser=parse_keytag_receipt)]
    pub receipt: Vec<(Keytag, Receipt)>,

    /// Emit machine-readable JSON instead of human-readable text
    #[arg(long, default_value_t = false)]
    pub output_json: bool,
}

impl Cmd {
    pub async fn run(self, config: &Config) -> anyhow::Result<()> {
        let connector = config.connector.connector().await?;
        let own_key = config.wallet.to_verification_key();
        let own_address = own_key.to_address(connector.network().into());
        let receipts = self.receipt.into_iter().collect::<BTreeMap<_, _>>();
        let preferences = AdaptorPreferences {
            min_single: 10_000,
            min_total: 1_000_000,
        };
        let protocol_parameters = connector.protocol_parameters().await?;
        let bounds = Bounds::around_tip(
            chain_now(&connector).await?,
            protocol_parameters.slot_length_ms(),
            Slot::new(Bounds::MARGIN_SLOTS),
        );
        let upper = bounds
            .upper
            .expect("a validity window always has an upper bound");
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
                    .await?,
            )
            .chain(
                connector
                    .utxos_at(&Credential::from_script(KONDUIT_VALIDATOR.hash), None)
                    .await?,
            )
            .collect();
        let mut tx = konduit_tx::adaptor::tx(
            &network_parameters,
            &preferences,
            &own_key,
            &receipts,
            &utxos,
            &upper,
        )?;
        let tx_id = tx.id().to_string();
        tx.sign(&config.wallet);
        connector.submit(&tx).await?;
        let output = OutputJson(self.output_json);
        if output.is_json() {
            let dto = output::TxOutput::from_id(tx_id);
            output::render(output, &dto)?;
        } else {
            println!("Tx id :: {}", tx_id);
        }
        Ok(())
    }
}
