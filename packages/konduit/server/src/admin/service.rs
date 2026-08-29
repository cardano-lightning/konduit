use std::{collections::BTreeMap, iter, sync::Arc};

use crate::{
    channel::{self, Retainer},
    db,
};
use async_trait::async_trait;
use cardano_connector::CardanoConnector;
use cardano_sdk::{Credential, Hash, Input, Output, SigningKey, VerificationKey};
use konduit_data::{Lock, Secret};
use konduit_tmp::{ChannelParameters, Keytag};
use konduit_tx::{
    Bounds, ChannelUtxo, KONDUIT_VALIDATOR, NetworkParameters, adaptor::AdaptorPreferences,
    to_verifying_key,
};

use super::{
    SyncApi,
    coiter::{CoItem, CoIter},
    config::Config,
};

#[derive(Clone)]
pub struct Service<Connector: CardanoConnector + Send + Sync + 'static> {
    bln: Arc<dyn bln_client::Api + Send + Sync + 'static>,
    cardano: Arc<Connector>,
    db: Arc<db::Db>,
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
        db: Arc<db::Db>,
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
        let host_utxos = cardano
            .utxos_at(&host_address.payment(), host_address.delegation().as_ref())
            .await?;
        let script_candidates = host_utxos
            .iter()
            .filter_map(|(input, output)| {
                output
                    .script()
                    .map(|script| (input.clone(), Hash::<28>::from(script), script.version()))
            })
            .collect::<Vec<_>>();
        let Some(script_utxo) = host_utxos.into_iter().find(|(_, o)| {
            o.script()
                .is_some_and(|s| Hash::<28>::from(s) == KONDUIT_VALIDATOR.hash)
        }) else {
            let script_summary = if script_candidates.is_empty() {
                "none".to_string()
            } else {
                script_candidates
                    .iter()
                    .map(|(input, hash, version)| {
                        format!("{input} hash={hash} version={version:#}")
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            };

            return Err(anyhow::anyhow!(
                "No reference script found at host address {}. Retrieved {} host UTxO(s); script candidates: {}. Expected script hash {}",
                host_address,
                script_candidates.len(),
                script_summary,
                KONDUIT_VALIDATOR.hash,
            ));
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
                constants.sub_vkey == to_verifying_key(own_vkey)
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

    async fn get_secrets(&self, locks: Vec<Lock>) -> Result<Vec<Secret>, anyhow::Error> {
        let mut secrets: Vec<Secret> = Vec::new();
        for lock in locks.into_iter() {
            if let bln_client::types::RevealResponse {
                secret: Some(secret),
            } = self
                .bln
                .reveal(bln_client::types::RevealRequest { lock: lock.0 })
                .await?
            {
                secrets.push(Secret(secret));
            }
        }
        Ok(secrets)
    }

    pub async fn unlocks(&self) -> Result<(), anyhow::Error> {
        // This is a silly implementation.
        let keytags = self.db.keys()?;
        for keytag in keytags.iter() {
            // FIXME : Race condition here
            let Some(channel) = self.db.get(keytag)? else {
                return Err(anyhow::anyhow!("Channel {} vanished", keytag));
            };
            let Some(locks) = channel
                .receipt()
                .as_ref()
                .map(|r| r.lockeds().map(|x| x.lock().to_owned()).collect::<Vec<_>>())
            else {
                continue;
            };
            let secrets = self.get_secrets(locks).await?;
            if secrets.is_empty() {
                continue;
            }
            self.db.update(keytag, channel::apply_secrets(secrets))?;
        }
        Ok(())
    }

    pub async fn sync_retainers(&self) -> Result<(), anyhow::Error> {
        // The suboptimal way.
        let snapshot = self.snapshot().await?;
        let left: Vec<Keytag> = self.db.keys()?;
        let right = self.retainers(&snapshot);
        for item in CoIter::new(left, right) {
            let (k, v) = match item {
                CoItem::Left(k) => (k, Vec::new()),
                CoItem::Right(k, v) | CoItem::Both(k, v) => (k, v),
            };
            self.db.upsert(&k, channel::upsert_retainers(v))?;
        }
        Ok(())
    }

    pub async fn claim(&self) -> Result<(), anyhow::Error> {
        let snapshot = self.snapshot().await?;
        let keys: Vec<Keytag> = self.db.keys()?;
        let receipts = keys
            .into_iter()
            .filter_map(|k| {
                let channel = self.db.get(&k).ok().flatten()?;
                let receipt = channel.receipt().as_ref()?;
                Some((k, receipt.to_owned()))
            })
            .collect::<BTreeMap<_, _>>();
        // FIXME :: This is the fudge. We treat tip as snapshot.
        // We are more likely to either:
        // - treat as confirmed something that will rollback
        // - use as an input a utxo that has already been spent.
        let tip = iter::once(self.script_utxo.clone())
            .chain(snapshot)
            .chain(self.wallet_utxos().await?)
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
        log::warn!("tx_id : {:?}", tx.id());
        self.cardano.submit(&tx).await?;
        Ok(())
    }

    pub async fn sync(&self) -> Result<(), anyhow::Error> {
        self.sync_retainers().await?;
        self.claim().await?;
        Ok(())
    }
}

#[async_trait(?Send)]
impl<Connector: CardanoConnector + Send + Sync + 'static> SyncApi for Service<Connector> {
    async fn sync(&self) -> Result<(), anyhow::Error> {
        Service::sync(self).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::admin::config::Config;
    use cardano_connector::CardanoConnector;
    use cardano_sdk::{
        Address, Credential, Hash, Input, Network, Output, PlutusScript, PlutusVersion,
        ProtocolParameters, SigningKey, Transaction, Value, address::kind, transaction::state,
    };
    use konduit_data::Duration;
    use konduit_tx::{KONDUIT_VALIDATOR, adaptor::AdaptorPreferences};
    use std::{collections::BTreeMap, sync::Arc};

    struct FakeConnector {
        network: Network,
        protocol_parameters: Result<ProtocolParameters, String>,
        host_utxos: Result<BTreeMap<Input, Output>, String>,
    }

    impl FakeConnector {
        fn new(
            protocol_parameters: Result<ProtocolParameters, impl Into<String>>,
            host_utxos: Result<BTreeMap<Input, Output>, impl Into<String>>,
        ) -> Self {
            Self {
                network: Network::Preview,
                protocol_parameters: protocol_parameters.map_err(Into::into),
                host_utxos: host_utxos.map_err(Into::into),
            }
        }
    }

    impl CardanoConnector for FakeConnector {
        fn network(&self) -> Network {
            self.network
        }

        async fn health(&self) -> anyhow::Result<String> {
            Ok("ok".to_string())
        }

        async fn protocol_parameters(&self) -> anyhow::Result<ProtocolParameters> {
            self.protocol_parameters.clone().map_err(anyhow::Error::msg)
        }

        async fn utxos_at(
            &self,
            payment: &Credential,
            delegation: Option<&Credential>,
        ) -> anyhow::Result<BTreeMap<Input, Output>> {
            let expected = test_host_address().payment();
            assert_eq!(payment, &expected);
            assert_eq!(delegation, test_host_address().delegation().as_ref());
            self.host_utxos.clone().map_err(anyhow::Error::msg)
        }

        async fn submit(
            &self,
            _transaction: &Transaction<state::ReadyForSigning>,
        ) -> anyhow::Result<()> {
            Ok(())
        }
    }

    fn tmp_db() -> db::Db {
        let file = tempfile::NamedTempFile::new().unwrap();
        db::Db::open(file.path().to_str().unwrap()).unwrap()
    }

    fn test_config() -> Config {
        let wallet = SigningKey::from([7; 32]);
        Config {
            wallet: wallet.clone(),
            channel_parameters: ChannelParameters {
                adaptor_key: wallet.to_verification_key(),
                close_period: Duration::from_secs(60),
                tag_length: 16,
            },
            tx_preferences: AdaptorPreferences {
                min_single: 1,
                min_total: 1,
            },
            host_address: test_host_address(),
        }
    }

    fn test_host_address() -> Address<kind::Shelley> {
        let payment = Credential::from_key(Hash::<28>::from([1; 28]));
        let delegation = Credential::from_key(Hash::<28>::from([2; 28]));
        Address::new(Network::Preview.into(), payment).with_delegation(delegation)
    }

    fn script_output() -> Output {
        Output::new(test_host_address().into(), Value::new(5_000_000)).with_plutus_script(
            PlutusScript::new(
                PlutusVersion::V3,
                KONDUIT_VALIDATOR.script.script().to_vec(),
            ),
        )
    }

    fn host_utxos_with_reference_script() -> BTreeMap<Input, Output> {
        BTreeMap::from([(Input::new(Hash::<32>::from([9; 32]), 0), script_output())])
    }

    #[tokio::test]
    async fn new_fails_when_protocol_parameters_cannot_be_loaded() {
        let connector = Arc::new(FakeConnector::new(
            Err("protocol parameters unavailable"),
            Ok::<_, &str>(host_utxos_with_reference_script()),
        ));

        let error = Service::new(
            test_config(),
            Arc::new(bln_client::mock::Client::new()),
            connector,
            Arc::new(tmp_db()),
        )
        .await
        .err()
        .expect("missing protocol parameters should fail startup");

        assert!(
            error
                .to_string()
                .contains("protocol parameters unavailable")
        );
    }

    #[tokio::test]
    async fn new_fails_when_reference_script_is_missing() {
        let connector = Arc::new(FakeConnector::new(
            Ok::<_, &str>(ProtocolParameters::default()),
            Ok::<_, &str>(BTreeMap::new()),
        ));

        let error = Service::new(
            test_config(),
            Arc::new(bln_client::mock::Client::new()),
            connector,
            Arc::new(tmp_db()),
        )
        .await
        .err()
        .expect("missing reference script should fail startup");

        let message = error.to_string();
        assert!(message.contains("No reference script found at host address"));
        assert!(message.contains("Retrieved 0 host UTxO(s)"));
    }

    #[tokio::test]
    async fn new_succeeds_with_protocol_parameters_and_reference_script() {
        let connector = Arc::new(FakeConnector::new(
            Ok::<_, &str>(ProtocolParameters::default()),
            Ok::<_, &str>(host_utxos_with_reference_script()),
        ));

        let service = Service::new(
            test_config(),
            Arc::new(bln_client::mock::Client::new()),
            connector,
            Arc::new(tmp_db()),
        )
        .await;

        assert!(service.is_ok(), "startup smoke path should succeed");
    }
}
