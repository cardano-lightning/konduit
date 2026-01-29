use cardano_connect_blockfrost::Blockfrost;
use cardano_tx_builder::{
    Credential, Datum, Hash, Input, Output, SigningKey, VerificationKey, address::kind,
};
use konduit_data::{Keytag, L1Channel};
use konduit_tx::{
    Bounds, KONDUIT_VALIDATOR, NetworkParameters, adaptor::AdaptorPreferences, filter_channels,
};
use std::{collections::BTreeMap, sync::Arc};

use crate::{admin::config::Config, channel::Retainer, common::ChannelParameters, db};

#[derive(Clone)]
pub struct Service {
    cardano: Arc<Blockfrost>,
    db: Arc<dyn db::Api + Send + Sync + 'static>,
    network_parameters: NetworkParameters,
    channel_parameters: ChannelParameters,
    tx_preferences: AdaptorPreferences,
    script_utxo: (Input, Output),
    wallet: SigningKey,
}

fn guard(cond: bool) -> Option<()> {
    if cond { Some(()) } else { None }
}

impl Service {
    pub async fn new(
        config: Config,
        cardano: Arc<Blockfrost>,
        db: Arc<dyn db::Api + Send + Sync + 'static>,
    ) -> anyhow::Result<Self> {
        let Config {
            wallet,
            channel_parameters,
            tx_preferences,
            host_address,
        } = config;
        // Treat network parameters as constants.
        // This will mean the service requires restarting
        // when a there is a protocol params change.
        let protocol_parameters = cardano.clone().protocol_parameters().await?;
        let network_id = cardano.network().into();
        let network_parameters = NetworkParameters {
            network_id,
            protocol_parameters,
        };
        // Treat reference script utxo as constant.
        // If this moves, the service needs to be restarted.
        let Some(script_utxo) = cardano
            .utxos_at(&host_address.payment(), host_address.delegation().as_ref())
            .await?
            .iter()
            .find(|(_, o)| {
                o.script()
                    .is_some_and(|s| Hash::<28>::from(s) == KONDUIT_VALIDATOR.hash)
            })
        else {
            return Err(anyhow::anyhow!("No reference script found"));
        };

        Ok(Self {
            cardano,
            db,
            network_parameters,
            channel_parameters,
            tx_preferences,
            script_utxo,
            wallet,
        })
    }

    fn retainers(&self, utxos: &BTreeMap<Input, Output>) -> BTreeMap<Keytag, Vec<Retainer>> {
        let close_period = self.close_period;
        let tag_length = self.max_tag_length;
        let own_vkey = VerificationKey::from(&self.skey);
        let candidates = filter_channels(&utxos, |co| {
            [
                co.constants.sub_vkey == own_vkey,
                co.constants.close_period >= close_period,
                co.constants.tag.0.len() <= tag_length,
                co.stage.is_opened(),
            ]
            .iter()
            .all(|&x| x)
        })
        .into_iter()
        .filter_map(|(_, co)| Retainer::try_from(&co).ok().map(|r| (co.keytag(), r)));
        let mut retainers = BTreeMap::new();
        for (keytag, retainer) in candidates {
            retainers
                .entry(keytag)
                .or_insert_with(Vec::new)
                .push(retainer);
        }
        retainers
    }

    /// These should be considered confirmed utxos,
    /// acceptable to be treated as retainers.
    pub async fn snapshot(&self) -> anyhow::Result<BTreeMap<Input, Output>> {
        let credential = Credential::from_script(KONDUIT_VALIDATOR.hash);
        let utxos = self.connector.utxos_at(&credential, None).await?;
        Ok(utxos)
    }

    pub async fn sync(&self) -> Result<(), anyhow::Error> {
        let snapshot = self.snapshot().await?;
        let retainers = self.retainers(&snapshot);
        let channels = self.db.update_retainers(retainers).await?;
        let receipts = channels
            .iter()
            .filter_map(|(kt, c)| c.ok().and_then(|c| c.receipt()).map(|r| (kt.clone(), r)))
            .collect::<BTreeMap<_, _>>();
        // FIXME :: This is the fudge. We treat tip as snapshot.
        // We are more likely to either:
        // - treat as confirmed something that will rollback
        // - use as an input a utxo that has already been spent.
        let tip = snapshot;
        let upper_bound = Bounds::twenty_mins();
        let tx = konduit_tx::adaptor::tx(
            self.network_parameters,
            self.tx_preferences,
            self.wallet,
            &receipts,
            &tip,
            upper_bound,
        );
        Ok(())
    }
}
