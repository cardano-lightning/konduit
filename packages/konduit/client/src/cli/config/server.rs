use std::path::Path;

use clap::Subcommand;

use crate::{config, server, server::codec};

use super::with_config;

#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// List known servers
    List,
    /// Add or replace a known server
    Add {
        base_url: String,
        #[arg(long, default_value = "cbor")]
        codec: codec::Kind,
    },
    /// Remove a known server by base_url
    Remove { base_url: String },
}

impl Cmd {
    pub fn run(&self, config_path: &Path) -> anyhow::Result<()> {
        if let Cmd::List = self {
            let cfg = config::Config::load(config_path)?;
            for s in cfg.servers() {
                println!("{} ({:?})", s.base_url(), s.codec());
            }
            return Ok(());
        }

        with_config(config_path, |cfg| {
            match self {
                Cmd::Add { base_url, codec } => {
                    cfg.add_server(server::Config::new(base_url.clone(), *codec));
                }
                Cmd::Remove { base_url } => {
                    if !cfg.remove_server(base_url) {
                        println!("no server found with base_url {base_url}");
                    }
                }
                Cmd::List => unreachable!("handled above"),
            }
            Ok(())
        })
    }
}
