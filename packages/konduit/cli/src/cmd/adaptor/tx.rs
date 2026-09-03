use crate::{cmd::parsers::parse_keytag_receipt, config::adaptor::Config};
use cardano_connector::CardanoConnector;
use cardano_sdk::Credential;
use konduit_data::AssetId;
use konduit_tmp::{Keytag, Receipt};
use konduit_tx::{
    self, Bounds, ChannelUtxo, KONDUIT_VALIDATOR, NetworkParameters, Utxos,
    adaptor::AdaptorPreferences, to_verifying_key,
};
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
}

impl Cmd {
    pub async fn run(self, config: &Config) -> anyhow::Result<()> {
        let connector = config.connector.connector().await?;
        let own_key = config.wallet.to_verification_key();
        let own_address = own_key.to_address(connector.network().into());
        let preferences_ada = (10_000, 1_000_000);
        let bounds = Bounds::twenty_mins();
        let upper = bounds.upper.unwrap();

        let protocol_parameters = connector.protocol_parameters().await?;
        let network_id = connector.network().into();
        let network_parameters = NetworkParameters {
            network_id,
            protocol_parameters,
        };
        let utxos: Utxos = connector
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
        let receipts = self
            .receipt
            .into_iter()
            .map(|(keytag, receipt)| {
                let mut assets = utxos
                    .iter()
                    .filter_map(|utxo| ChannelUtxo::try_from(utxo).ok())
                    .filter(|channel| {
                        channel.data().keytag() == keytag
                            && channel.data().constants().sub_vkey == to_verifying_key(own_key)
                    })
                    .map(|channel| channel.data().constants().asset.clone());
                let asset = assets
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("no channel matches receipt {keytag}"))?;
                if assets.any(|candidate| candidate != asset) {
                    anyhow::bail!("receipt {keytag} matches channels with different assets");
                }
                Ok((keytag, (asset, receipt)))
            })
            .collect::<anyhow::Result<BTreeMap<_, _>>>()?;
        let preferences = AdaptorPreferences {
            min_single: preferences_ada.0,
            min_total: preferences_ada.1,
            asset_minimums: receipts
                .values()
                .filter_map(|(asset, _)| {
                    (*asset != AssetId::Ada).then(|| (asset.clone(), preferences_ada))
                })
                .collect(),
        };
        let mut tx = konduit_tx::adaptor::tx(
            &network_parameters,
            &preferences,
            &own_key,
            &receipts,
            &utxos,
            &upper,
        )?;
        println!("Tx id :: {}", tx.id());
        tx.sign(&config.wallet);
        connector.submit(&tx).await
    }
}
