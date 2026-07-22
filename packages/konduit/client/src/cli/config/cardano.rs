use std::path::Path;

use cardano_sdk::Network;
use clap::Subcommand;

use crate::cardano;
use crate::config;

use super::with_config;

#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// Print the current cardano connector config
    Show,
    /// Use Cardano Connector Client (proxy)
    SetClient {
        network: Network,
        #[arg(long)]
        url: String,
    },
    /// Use Blockfrost as the connector backend
    SetBlockfrost {
        network: Network,
        #[arg(long)]
        project_id: Option<String>,
    },
    /// Use UTxO RPC as the connector backend
    SetUtxoRpc {
        network: Network,
        #[arg(long)]
        uri: Option<String>,
    },
}

impl Cmd {
    pub fn run(&self, config_path: &Path) -> anyhow::Result<()> {
        if let Cmd::Show = self {
            let cfg = config::Config::load(config_path)?;
            println!("{:?}", cfg.cardano());
            return Ok(());
        }

        with_config(config_path, |cfg| {
            match self {
                Cmd::SetClient { network, url } => {
                    cfg.set_cardano(cardano::Config::Client(cardano::config::Client {
                        network: *network,
                        base_url: url.clone(),
                    }));
                }
                Cmd::SetBlockfrost {
                    network,
                    project_id,
                } => {
                    cfg.set_cardano(cardano::Config::Blockfrost(cardano::config::Blockfrost {
                        network: *network,
                        project_id: project_id.clone(),
                    }));
                }
                Cmd::SetUtxoRpc { network, uri } => {
                    cfg.set_cardano(cardano::Config::UtxoRpc(cardano::config::UtxoRpc {
                        network: *network,
                        uri: uri.clone(),
                    }));
                }
                Cmd::Show => unreachable!("handled above"),
            }
            Ok(())
        })
    }
}
