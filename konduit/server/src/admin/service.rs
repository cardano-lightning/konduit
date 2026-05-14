#![allow(dead_code)] // Admin service pending rewrite against konduit-indexer.

use crate::{
    admin::{SyncApi, config::Config},
    channel::Retainer,
    db,
};
use async_trait::async_trait;
use cardano_connector::CardanoConnector;
use cardano_sdk::{Credential, Hash, Input, Output, SigningKey, VerificationKey};
use konduit_data::{ChannelParameters, Keytag, Secret};
use konduit_tx::{
    Bounds, ChannelUtxo, KONDUIT_VALIDATOR, NetworkParameters, adaptor::AdaptorPreferences,
};
use std::{collections::BTreeMap, iter, sync::Arc};

#[derive(Clone)]
pub struct Service<Connector: CardanoConnector + Send + Sync + 'static> {
    bln: Arc<dyn bln_client::Api + Send + Sync + 'static>,
    cardano: Arc<Connector>,
    db: Arc<dyn db::Api + Send + Sync + 'static>,
    network_parameters: NetworkParameters,
    channel_parameters: ChannelParameters,
    tx_preferences: AdaptorPreferences,
    script_utxo: (Input, Output),
    wallet: SigningKey,
}

impl<Connector: CardanoConnector + Send + Sync + 'static> Service<Connector> {
    pub async fn new(
        config: Config,
        bln: Arc<dyn bln_client::Api + Send + Sync + 'static>,
        cardano: Arc<Connector>,
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
            .into_iter()
            .find(|(_, o)| {
                o.script()
                    .is_some_and(|s| Hash::<28>::from(s) == KONDUIT_VALIDATOR.hash)
            })
        else {
            return Err(anyhow::anyhow!("No reference script found"));
        };

        Ok(Self {
            bln,
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
        let close_period = self.channel_parameters.close_period;
        let tag_length = self.channel_parameters.tag_length;
        let own_vkey = VerificationKey::from(&self.wallet);
        let candidates = utxos
            .iter()
            .filter_map(|u| ChannelUtxo::try_from(u).ok())
            .filter(|u| {
                let channel = u.data();
                let constants = channel.constants();
                constants.sub_vkey == own_vkey
                    && constants.close_period >= close_period
                    && constants.tag.len() <= tag_length
                    && channel.stage().is_opened()
            })
            .filter_map(|u| {
                Retainer::try_from(u.data())
                    .ok()
                    .map(|r| (u.data().keytag(), r))
            });
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
    async fn snapshot(&self) -> anyhow::Result<BTreeMap<Input, Output>> {
        let credential = Credential::from_script(KONDUIT_VALIDATOR.hash);
        let utxos = self.cardano.utxos_at(&credential, None).await?;
        Ok(utxos)
    }

    /// These should be considered confirmed utxos,
    /// acceptable to be treated as retainers.
    async fn wallet_utxos(&self) -> anyhow::Result<BTreeMap<Input, Output>> {
        let vkh = Hash::<28>::new(VerificationKey::from(&self.wallet));
        let credential = Credential::from_key(vkh);
        let utxos = self.cardano.utxos_at(&credential, None).await?;
        Ok(utxos)
    }

    pub async fn unlocks(&self) -> Result<(), anyhow::Error> {
        let channels = self.db.get_all().await?;
        for (keytag, channel) in channels.iter() {
            if let Some(receipt) = channel.receipt() {
                for locked in receipt.lockeds().iter() {
                    if let bln_client::types::RevealResponse {
                        secret: Some(secret),
                    } = self
                        .bln
                        .reveal(bln_client::types::RevealRequest {
                            lock: locked.lock().0,
                        })
                        .await?
                    {
                        let s = Secret(secret);
                        self.db
                            .update_channel(keytag, Box::new(move |ch| ch.apply_unlock(s)))
                            .await?;
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn sync(&self) -> Result<(), anyhow::Error> {
        // TODO: rewrite admin sync against konduit-channel Backing API.
        // The retainer/update_retainers model has been removed from the DB layer.
        // Until konduit-indexer is implemented, sync builds the adaptor TX
        // from the current channel state without updating backing.
        let snapshot = self.snapshot().await?;
        let receipts = self
            .db
            .get_all()
            .await?
            .into_iter()
            .filter_map(|(kt, ch)| ch.receipt().cloned().map(|r| (kt, r)))
            .collect::<BTreeMap<_, _>>();
        let tip = iter::once(self.script_utxo.clone())
            .chain(snapshot.into_iter())
            .chain(self.wallet_utxos().await?.into_iter())
            .collect::<BTreeMap<_, _>>();
        let upper_bound = Bounds::twenty_mins().upper.expect("This returns `Some`!!");
        let mut tx = konduit_tx::adaptor::tx(
            &self.network_parameters,
            &self.tx_preferences,
            &VerificationKey::from(&self.wallet),
            &receipts,
            &tip,
            &upper_bound,
        )?;
        tx.sign(&self.wallet);
        self.cardano.submit(&tx).await?;
        Ok(())
    }
}

#[async_trait(?Send)]
impl<Connector: CardanoConnector + Send + Sync + 'static> SyncApi for Service<Connector> {
    async fn sync(&self) -> Result<(), anyhow::Error> {
        Service::sync(self).await
    }
}
