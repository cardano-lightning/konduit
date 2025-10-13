use anyhow::{Result, anyhow};
use num::rational::Ratio;
use std::collections::BTreeMap;

use cardano_tx_builder::{Credential, Input, NetworkId, Output, ProtocolParameters};

use cardano_connect::CardanoConnect;

use blockfrost::BlockfrostAPI;

mod time;
// use blockfrost::{BlockfrostAPI, Pagination};
// use blockfrost_openapi::models;

const UNIT_LOVELACE: &str = "lovelace";

const MAINNET_PREFIX: &str = "mainnet";
const PREPROD_PREFIX: &str = "preprod";
const PREVIEW_PREFIX: &str = "preview";

pub struct Blockfrost {
    api: BlockfrostAPI,
    base_url: String,
    client: reqwest::Client,
    network: NetworkId,
    project_id: String,
}

impl Blockfrost {
    pub fn new(project_id: String) -> Self {
        let network_prefix = if project_id.starts_with(MAINNET_PREFIX) {
            MAINNET_PREFIX.to_string()
        } else if project_id.starts_with(PREPROD_PREFIX) {
            PREPROD_PREFIX.to_string()
        } else if project_id.starts_with(PREVIEW_PREFIX) {
            PREVIEW_PREFIX.to_string()
        } else {
            panic!("unexpected project id prefix")
        };
        let base_url = format!("https://cardano-{}.blockfrost.io/api/v0", network_prefix,);
        let api = BlockfrostAPI::new(project_id.as_str(), Default::default());
        Self {
            api,
            base_url,
            client: reqwest::Client::new(),
            network: if project_id.starts_with(MAINNET_PREFIX) {
                NetworkId::mainnet()
            } else {
                NetworkId::testnet()
            },
            project_id,
        }
    }
}

impl CardanoConnect for Blockfrost {
    fn network(&self) -> NetworkId {
        self.network.clone()
    }

    async fn health(&self) -> Result<String> {
        match self.api.health().await {
            Ok(x) => Ok(format!("{:?}", x)),
            Err(y) => Err(anyhow!(y.to_string())),
        }
    }

    async fn protocol_parameters(&self) -> Result<ProtocolParameters> {
        let x = self.api.epochs_latest_parameters().await?;

        let pp = ProtocolParameters::default()
            .with_fee_per_byte(x.min_fee_a as u64)
            .with_fee_constant(x.min_fee_b as u64)
            .with_collateral_coefficient(
                x.collateral_percent
                    .ok_or(anyhow!("Expect `collateral_percent`"))? as f64
                    / 100.0,
            )
            .with_referenced_scripts_base_fee_per_byte(
                x.min_fee_ref_script_cost_per_byte
                    .ok_or(anyhow!("Expect `min_fee_ref_script_cost_per_byte`"))?
                    .round() as u64,
            )
            .with_referenced_scripts_fee_multiplier(Ratio::new(12, 10)) // Not in response
            .with_referenced_scripts_fee_step_size(25000) // Not in response
            .with_execution_price_mem(x.price_mem.ok_or(anyhow!("Expect `price_mem`"))?)
            .with_execution_price_cpu(x.price_step.ok_or(anyhow!("Expect `price_step`"))?)
            // FIXME :: Timeslots from mainnet
            .with_start_time(1506203091) // Not in response
            .with_first_shelley_slot(4492800) // Not in response
            .with_plutus_v3_cost_model(
                x.cost_models_raw
                    .ok_or(anyhow!("Expect `cost_models_raw`"))?
                    .ok_or(anyhow!("Expect `cost_models_raw`"))?
                    .get("PlutusV3")
                    .ok_or(anyhow!("Expect `cost_models_raw.PlutusV3`"))?
                    .as_array()
                    .ok_or(anyhow!("Expect array"))?
                    .iter()
                    .map(|x| {
                        x.as_number()
                            .ok_or(anyhow!("Expect Number"))
                            .and_then(|x| x.as_i64().ok_or(anyhow!("Expect i64")))
                    })
                    .collect::<Result<Vec<i64>>>()?,
            );
        Ok(pp)
    }

    async fn utxos_at(
        &self,
        payment: &Credential,
        delegation: &Option<Credential>,
    ) -> Result<BTreeMap<Input, Output>> {
        todo!()
    }

    async fn submit(&self, tx: Vec<u8>) -> Result<String> {
        todo!()
    }
}
