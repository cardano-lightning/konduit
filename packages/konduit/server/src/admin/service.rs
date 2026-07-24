use crate::{
    admin::{SyncApi, config::Config},
    channel::Retainer,
    db,
};
use async_trait::async_trait;
use cardano_connector::CardanoConnector;
use cardano_sdk::{Credential, Hash, Input, Output, SigningKey, VerificationKey};
use konduit_data::Secret;
use konduit_tmp::{ChannelParameters, Keytag};
use konduit_tx::{
    Bounds, ChannelUtxo, KONDUIT_VALIDATOR, NetworkParameters, adaptor::AdaptorPreferences,
    to_verifying_key,
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

    pub async fn unlocks(&self) -> Result<(), anyhow::Error> {
        // This is a silly implementation.
        let channels = self.db.get_all().await?;
        for (keytag, channel) in channels.iter() {
            if let Some(lockeds) = channel.receipt().map(|x| x.lockeds().collect::<Vec<_>>()) {
                for locked in lockeds.iter() {
                    if let bln_client::types::RevealResponse {
                        secret: Some(secret),
                    } = self
                        .bln
                        .reveal(bln_client::types::RevealRequest {
                            lock: locked.lock().0,
                        })
                        .await?
                    {
                        self.db.unlock(keytag, Secret(secret)).await?;
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn sync(&self) -> Result<(), anyhow::Error> {
        // FIXME :: Sync BLN
        // At present this is not even in the admin context
        let snapshot = self.snapshot().await?;
        let retainers = self.retainers(&snapshot);
        let channels = self.db.update_retainers(retainers).await?;
        let receipts = channels
            .iter()
            .filter_map(|(kt, c)| {
                c.as_ref()
                    .ok()
                    .and_then(|c| c.receipt())
                    .map(|r| (kt.clone(), r))
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

#[cfg(test)]
mod tests {
    use super::Service;
    use crate::{Channel, ChannelError, admin::config::Config, channel::Retainer, db};
    use async_trait::async_trait;
    use cardano_connector::CardanoConnector;
    use cardano_sdk::{
        Address, Credential, Hash, Input, Network, Output, PlutusScript, PlutusVersion,
        ProtocolParameters, SigningKey, Transaction, Value, address::kind, transaction::state,
    };
    use konduit_data::{ChannelParameters, Duration, Keytag, Locked, Secret, Squash};
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

    struct FakeDb;

    #[async_trait]
    impl db::Api for FakeDb {
        async fn update_retainers(
            &self,
            _retainers: BTreeMap<Keytag, Vec<Retainer>>,
        ) -> db::Result<BTreeMap<Keytag, Result<Channel, ChannelError>>> {
            Ok(BTreeMap::new())
        }

        async fn get_channel(&self, _keytag: &Keytag) -> db::Result<Option<Channel>> {
            Ok(None)
        }

        async fn get_all(&self) -> db::Result<BTreeMap<Keytag, Channel>> {
            Ok(BTreeMap::new())
        }

        async fn update_squash(&self, _keytag: &Keytag, _squash: Squash) -> db::Result<Channel> {
            unreachable!("db should not be used during Service::new tests")
        }

        async fn append_locked(&self, _keytag: &Keytag, _locked: Locked) -> db::Result<Channel> {
            unreachable!("db should not be used during Service::new tests")
        }

        async fn unlock(&self, _keytag: &Keytag, _secret: Secret) -> db::Result<Channel> {
            unreachable!("db should not be used during Service::new tests")
        }
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
            Arc::new(FakeDb),
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
            Arc::new(FakeDb),
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
            Arc::new(FakeDb),
        )
        .await;

        assert!(service.is_ok(), "startup smoke path should succeed");
    }
}
