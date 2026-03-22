mod secret;
use cardano_connector::CardanoConnector;
use cardano_sdk::{SigningKey};
use futures::future::join_all;
use konduit_tx::KONDUIT_VALIDATOR;
use secret::*;

mod adaptor;
mod bln;
mod cardano;
pub(crate) mod consumer;

use std::{collections::HashMap, fs, iter, path::Path};

use serde::{Deserialize, Serialize};

use crate::{Pool, Signer, env, l1};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub bln: HashMap<String, bln::Config>,
    pub adaptor: HashMap<String, adaptor::Config>,
    pub consumer: HashMap<String, consumer::Config>,
    /// "Fuel" wallet
    pub wallet: SecretKey,
    /// We only need one, but just in case.
    pub cardano: HashMap<String, cardano::Config>,
    // pub settings: Settings,
}

impl Config {
    /// Helper to process a map of secrets with a consistent naming convention.
    fn process_map<T: Secret>(
        prefix: &str,
        type_label: &str,
        map: &mut HashMap<String, T>,
        mut action: impl FnMut(&mut T, &str),
    ) {
        for (name, item) in map.iter_mut() {
            Self::process_one(prefix, type_label, name.as_str(), item, &mut action);
        }
    }

    fn process_one<T: Secret>(
        prefix: &str,
        type_label: &str,
        name: &str,
        item: &mut T,
        mut action: impl FnMut(&mut T, &str),
    ) {
        let item_prefix = if prefix.is_empty() && name.is_empty() {
            format!("{}", type_label)
        } else if prefix.is_empty() {
            format!("{}_{}", type_label, name.to_uppercase())
        } else {
            format!("{}_{}", prefix, name.to_uppercase())
        };
        action(item, &item_prefix);
    }

    pub fn example() -> Self {
        Self {
            bln: bln::Config::examples(),
            adaptor: adaptor::Config::examples(),
            consumer: consumer::Config::examples(),
            wallet: SecretKey::generate(),
            cardano: cardano::Config::examples(),
        }
    }

    fn wallet(&self) -> SigningKey {
        self.wallet.inner.as_ref().expect("No wallet key given").clone().into_signing_key()
    }

    pub async fn build(self) -> anyhow::Result<Pool> {
        let wallet = self.wallet();

        let cardano_futures = self.cardano.into_iter().map(|(name, val)| async move {
            let built_val = val.build().await?;
            Ok((name, built_val))
        });

        let cardano: HashMap<_, _> = join_all(cardano_futures)
            .await
            .into_iter()
            .collect::<anyhow::Result<_>>()?;

        let cardano =  
            cardano.into_values().next().expect("At least one cardano connector required");

        let network_id = cardano.network().into();
        let fuel_address = wallet.to_verification_key().to_address(network_id);

        let bln = self
            .bln
            .into_iter()
            .map(|(name, val)| val.build().map(|val| (name, val)))
            .collect::<anyhow::Result<HashMap<_, _>>>()?;


        let adaptor = self
            .adaptor
            .into_iter()
            .map(|(name, val)| (name, val.build()))
            .collect::<HashMap<_, _>>();

        let mut host_address = fuel_address.clone();

        for v in adaptor.iter() {
            let info = v.1.info().await?;
            if KONDUIT_VALIDATOR.hash != info.tx_help.validator {
                return Err(anyhow::anyhow!("Adaptor {} has a different validator version", v.0))
            }
            host_address = info.tx_help.host_address;
        }

        let signer = Signer::from(
            self.consumer.values().map(|x| x.key())
            .chain(iter::once(self.wallet.inner.as_ref().unwrap().clone().into_signing_key())
        ));

        let mut l1 = l1::L1::new(
            cardano,
            signer,
            fuel_address,
            host_address,
        ).await?;

        let _ = l1.sync().await?;

        let channels = self.consumer.iter("")

        Pool::new(bln, adaptor, l1)
    }
}

impl Secret for Config {
    fn inject(&mut self, prefix: &str) {
        Self::process_map(prefix, "ADAPTOR", &mut self.adaptor, |v, p| v.inject(p));
        Self::process_map(prefix, "BLN", &mut self.bln, |v, p| v.inject(p));
        Self::process_map(prefix, "CARDANO", &mut self.cardano, |v, p| v.inject(p));
        Self::process_map(prefix, "CONSUMER", &mut self.consumer, |v, p| v.inject(p));
        Self::process_one(prefix, "WALLET", "", &mut self.wallet, |v, p| v.inject(p));
    }

    fn extract(&mut self, prefix: &str, env_list: &mut Vec<String>) {
        Self::process_map(prefix, "ADAPTOR", &mut self.adaptor, |v, p| {
            v.extract(p, env_list)
        });
        Self::process_map(prefix, "BLN", &mut self.bln, |v, p| v.extract(p, env_list));
        Self::process_map(prefix, "CARDANO", &mut self.cardano, |v, p| {
            v.extract(p, env_list)
        });
        Self::process_map(prefix, "CONSUMER", &mut self.consumer, |v, p| {
            v.extract(p, env_list)
        });
        Self::process_one(prefix, "WALLET", "", &mut self.wallet, |v, p| {
            v.extract(p, env_list)
        });
    }
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        env::load(&path)?;
        let content = fs::read_to_string(path)?;
        let mut config: Config = toml::from_str(&content)?;
        config.inject("");
        Ok(config)
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let mut env_list = Vec::new();
        let mut config_copy = self.clone();
        config_copy.extract("", &mut env_list);
        let toml_content = toml::to_string_pretty(&config_copy)?;
        fs::write(&path, toml_content)?;
        env::save(&path, env_list)?;
        Ok(())
    }
}
