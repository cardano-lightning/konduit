use anyhow::anyhow;
use cardano_connect::CardanoConnect;

#[cfg(feature = "blockfrost")]
pub(crate) fn new() -> anyhow::Result<impl CardanoConnect> {
    use cardano_connect_blockfrost::Blockfrost;

    let project_id = std::env::var(crate::env::BLOCKFROST_PROJECT_ID).map_err(|e| {
        anyhow!(e).context(format!(
            "missing {} environment variable",
            crate::env::BLOCKFROST_PROJECT_ID
        ))
    })?;

    Ok(Blockfrost::new(project_id))
}

#[cfg(not(feature = "blockfrost"))]
pub(crate) fn new() -> anyhow::Result<impl CardanoConnect> {
    Err(anyhow!(
        "no Cardano connector configured; did you forget to choose one when compiling?"
    ))
}
