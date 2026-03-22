use std::collections::HashMap;

use bln_client::MerchantApi;
use cardano_connector::CardanoConnector;
use http_client_native::HttpClient;

use crate::{Channel, adaptor, config, l1::L1};

// Pool of clients
pub struct Pool {
    pub bln: HashMap<String, Box<dyn MerchantApi>>,
    pub adaptor: HashMap<String, adaptor::Adaptor<HttpClient>>,
    pub l1: L1,
    pub channel: HashMap<String, Channel<'static>>,
}

impl Pool {
    pub fn new(
        bln: HashMap<String, Box<dyn MerchantApi>>,
        adaptor: HashMap<String, adaptor::Adaptor<HttpClient>>,
        l1: L1,
        // consumer: HashMap<String, config::consumer::Config>,
        // wallet: HashMap<String, Wallet>,
        // settings: Settings,
    ) -> anyhow::Result<Self> {
        //     for ch in value.channel().iter() {}
        // }

        Ok(Self {
            bln,
            adaptor,
            l1,
            channel : HashMap::new(),
        })
    }

    pub async fn health(&self) -> anyhow::Result<()> {
        // TODO :: Print single pretty json
        println!("BLN");
        for (name, conn) in self.bln.iter() {
            println!(
                "  {} :: {}",
                name,
                conn.health().await.unwrap_or("FAIL".to_string())
            );
        }
        println!("ADAPTOR");
        for (name, conn) in self.adaptor.iter() {
            println!(
                "  {} :: {}",
                name,
                conn.info()
                    .await
                    .map(|x| serde_json::to_string_pretty(&x).unwrap())
                    .unwrap_or("FAIL".to_string())
            );
        }
        println!("L1");
        println!("  {}", self.l1.info());

        Ok(())
    }

    pub async fn tip(&self) -> anyhow::Result<()> {
        todo!()
    }
}
