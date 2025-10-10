use anyhow::Result;

use cardano_tx_builder::{Credential, NetworkId, ProtocolParameters, ResolvedInput};

use cardano_connect::CardanoConnect;

use blockfrost::{BlockfrostAPI, Pagination};
use blockfrost_openapi::models;

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
