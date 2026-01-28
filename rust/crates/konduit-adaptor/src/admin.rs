use cardano_connect_blockfrost::Blockfrost;
use cardano_tx_builder::{
    Address, Credential, Datum, Hash, Input, Output, SigningKey, VerificationKey, address::kind,
};
use konduit_data::{Duration, Keytag, L1Channel};
use konduit_tx::{
    KONDUIT_VALIDATOR, NetworkParameters, adaptor::AdaptorPreferences, filter_channels,
};
use std::{collections::BTreeMap, sync::Arc};

use crate::{common::ChannelParameters, db};

mod args;
mod config;
pub use args::Args as AdminArgs;

#[derive(Clone)]
pub struct Admin {
    cardano: Arc<Blockfrost>,
    db: Arc<dyn db::DbInterface + Send + Sync + 'static>,
    network_parameters: NetworkParameters,
    channel_parameters: ChannelParameters,
    tx_preferences: AdaptorPreferences,
    script_utxo: (Input, Output),
    wallet: SigningKey,
}

fn guard(cond: bool) -> Option<()> {
    if cond { Some(()) } else { None }
}

impl Admin {
    pub async fn new(
        config: AdminConfig,
        cardano: Arc<Blockfrost>,
        db: Arc<dyn db::DbInterface + Send + Sync + 'static>,
    ) -> anyhow::Result<Admin> {
        let AdminConfig {
            wallet,
            channel_parameters,
            preferences,
            host_address,
        } = config;
        // Treat network parameters as constants.
        // This will mean the service requires restarting
        // when a there is a protocol params change.
        let protocol_parameters = cardano.protocol_parameters().await?;
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

    fn parse_channel_output(&self, output: Output) -> Option<(Keytag, L1Channel)> {
        let datum = output.datum()?;
        match datum {
            Datum::Inline(plutus_data) => {
                let datum = konduit_data::Datum::try_from(plutus_data).ok()?;
                guard(datum.own_hash == self.script_hash)?;
                guard(output.value().assets().is_empty())?;
                let adaptor_verification_key = VerificationKey::from(&self.skey);
                guard(datum.constants.sub_vkey == adaptor_verification_key)?;
                guard(datum.constants.close_period >= self.close_period)?;
                guard({
                    let bytes = <Vec<u8>>::from(&datum.constants.tag);
                    bytes.len() <= self.max_tag_length
                })?;
                let keytag = Keytag::new(datum.constants.add_vkey, datum.constants.tag);
                let res = (
                    keytag,
                    L1Channel {
                        stage: datum.stage,
                        amount: output
                            .value()
                            .lovelace()
                            .checked_sub(MIN_ADA_BUFFER)
                            .unwrap_or(0),
                    },
                );
                Some(res)
            }
            Datum::Hash(_) => None,
        }
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

    async fn fetch_tip(&self) -> anyhow::Result<TipBody> {
        let script_credential = Credential::from_script(KONDUIT_VALIDATOR.hash);
        let channel_utxos = self.connector.utxos_at(&script_credential, None).await?;
        let close_period = self.close_period;
        let tag_length = self.max_tag_length;
        let own_vkey = VerificationKey::from(&self.skey);
        let candidates = filter_channels(&channel_utxos, |co| {
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
        .map(|(_, co)| (co.keytag(), co.to_l1_channel()));
        let mut tip_body = BTreeMap::new();
        for (keytag, retainer) in candidates {
            tip_body
                .entry(keytag)
                .or_insert_with(Vec::new)
                .push(retainer);
        }
        Ok(tip_body)
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
        let tx = konduit_tx::adaptor::tx(
            network_parameters,
            preferences,
            wallet,
            &receipts,
            &tip,
            upper_bound,
        );
        Ok(())
    }
}
