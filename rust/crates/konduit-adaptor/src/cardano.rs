use anyhow::anyhow;
use cardano_connect::CardanoConnect;
use cardano_connect_blockfrost::Blockfrost;

// FIXME :: Cannot make impl dyn, so for not use concrete impl
pub type Cardano = Blockfrost;

pub(crate) async fn new() -> anyhow::Result<Cardano> {
    let project_id = std::env::var(crate::env::BLOCKFROST_PROJECT_ID).map_err(|e| {
        anyhow!(e).context(format!(
            "missing {} environment variable",
            crate::env::BLOCKFROST_PROJECT_ID
        ))
    })?;

    let client = Blockfrost::new(project_id);
    let resp = client.health().await;
    match resp {
        Ok(_) => Ok(client),
        Err(e) => Err(e),
    }
}
