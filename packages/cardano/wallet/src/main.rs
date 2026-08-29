use cardano_connector_direct::Blockfrost;
use cardano_sdk::{NetworkId, ProtocolParameters};
use cardano_wallet::{Cmd, Embedded, Wallet};
use clap::Parser;
use figment::{
    Figment,
    providers::{Env, Format, Toml},
};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};

const DEFAULT_CONFIG_PATH: &str = "cardano-wallet-config.toml";

/// Connector + signing key. `init` writes `Config::default()` here as a
/// starting template; other commands load it via figment (TOML file +
/// `CARDANO_WALLET_*` env overrides).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct Config {
    connector: ConnectorConfig,
    signing_key: cardano_wallet::Config,
}

/// Only Blockfrost for now.
#[derive(Debug, Clone, Serialize, Deserialize)]
enum ConnectorConfig {
    Blockfrost { project_id: String },
}

impl Default for ConnectorConfig {
    fn default() -> Self {
        Self::Blockfrost {
            project_id: "mainnetYourProjectIdHere".into(),
        }
    }
}

#[derive(Parser)]
struct Cli {
    #[arg(long, default_value = DEFAULT_CONFIG_PATH)]
    config: PathBuf,
    /// JSON file to load protocol parameters from. Guessed from the
    /// wallet's network id if omitted.
    #[arg(long)]
    protocol_parameters: Option<PathBuf>,
    #[command(subcommand)]
    cmd: Cmd,
}

/// Mainnet exact; anything else (testnet variants aren't distinguishable
/// via `NetworkId` alone) assumed preprod.
fn guess_protocol_parameters(network_id: NetworkId) -> ProtocolParameters {
    match network_id {
        NetworkId::MAINNET => ProtocolParameters::mainnet(),
        _ => ProtocolParameters::preprod(),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if matches!(cli.cmd, Cmd::Init) {
        std::fs::write(&cli.config, toml::to_string_pretty(&Config::default())?)?;
        println!("wrote {}", cli.config.display());
        return Ok(());
    }

    let config: Config = Figment::new()
        .merge(Toml::file(&cli.config))
        .merge(Env::prefixed("CARDANO_WALLET_"))
        .extract()?;

    let ConnectorConfig::Blockfrost { project_id } = config.connector;
    let connector = Arc::new(Blockfrost::new(project_id));
    let wallet = Embedded::new(connector, config.signing_key, None);

    let protocol_parameters = match &cli.protocol_parameters {
        Some(path) => serde_json::from_str(&std::fs::read_to_string(path)?)?,
        None => guess_protocol_parameters(wallet.network_id().await?),
    };

    let output = cli.cmd.run(&wallet, &protocol_parameters).await?;

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
