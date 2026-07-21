use clap::{Parser, Subcommand};
use std::path::PathBuf;

pub mod config;
pub mod env;
pub mod keygen;

#[derive(Debug, Parser)]
#[command(author, version, about = "Konduit Consumer CLI")]
pub struct Cli {
    /// Path to the konduit.toml config file
    #[arg(
        long,
        env = "KONDUIT_CONFIG",
        default_value = "./konduit.toml",
        global = true
    )]
    pub config: PathBuf,

    // /// Path to cache file
    // #[arg(
    //     long,
    //     env = "KONDUIT_CACHE",
    //     default_value = "./konduit-cache.json",
    //     global = true
    // )]
    // pub cache: Path,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Generate a fresh random 32-byte key, hex-encoded
    Keygen,
    /// Get and set values in konduit.toml
    #[clap(subcommand)]
    Config(config::Cmd),
}

impl Cli {
    pub fn run(self) -> anyhow::Result<()> {
        match self.command {
            Commands::Keygen => keygen::run(),
            Commands::Config(cmd) => cmd.run(&self.config),
        }
    }
}
