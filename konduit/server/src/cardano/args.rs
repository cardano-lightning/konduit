use cardano_connector::CardanoConnector;
use cardano_connector_direct::Blockfrost;

#[derive(Debug, Clone, clap::Args)]
pub struct CardanoArgs {
    // Use blockfrost_project_id. The network is inferred, and the
    // URL is assumed to be blockfrost.io's
    #[arg(long, env = crate::env::BLOCKFROST_PROJECT_ID)]
    pub blockfrost_project_id: Option<String>,
}

impl CardanoArgs {
    pub async fn build(&self) -> anyhow::Result<super::Cardano> {
        if let Some(project_id) = &self.blockfrost_project_id {
            let client = Blockfrost::new(project_id.clone());
            let Ok(_) = client.health().await else {
                return Err(anyhow::anyhow!("Cardano health check failed"));
            };
            Ok(client)
        } else {
            Err(anyhow::anyhow!("Cardano connect args are missing"))
        }
    }
}
