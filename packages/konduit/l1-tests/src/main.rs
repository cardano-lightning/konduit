use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use cardano_connector::CardanoConnector;
use clap::{Args, Parser, Subcommand};

// Adjust this if `config.rs` isn't a sibling module of `main.rs` — e.g. if it
// lives in the `konduit_l1_tests` lib crate, use:
//   use konduit_l1_tests::config::Config;
mod config;
use config::Config;

mod show;
use show::Show;

mod strategy;
use strategy::StepStrategy;
mod receipt;
mod tx;

#[derive(Parser, Debug)]
#[command(name = "konduit-l1-tests", version, about, long_about = None)]
pub struct Cli {
    /// Path to the config file.
    #[arg(
        long,
        env = "L1_TESTS_CONFIG",
        default_value = "config.toml",
        global = true
    )]
    config: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Generate a default config and write it to the config path.
    Init(InitArgs),
    /// Load and display the current config.
    Show(ShowArgs),
    /// Run the test suite.
    Tx(TxArgs),
}

impl Cli {
    /// Dispatches to whichever subcommand was selected. `init` only needs
    /// the config *path* (it's writing a fresh one); everything else needs
    /// the actual loaded `Config`, owned, so we load it inline and hand
    /// ownership down.
    pub async fn run(self) -> anyhow::Result<()> {
        match self.command {
            Command::Init(args) => args.run(&self.config).await,
            Command::Show(args) => args.run(load(&self.config)?).await,
            Command::Tx(args) => args.run(load(&self.config)?).await,
        }
    }
}

/// Reads and parses the config file at `path` into a `Config`.
fn load(path: &Path) -> anyhow::Result<Config> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read config at {}", path.display()))?;
    toml::from_str(&raw).with_context(|| format!("failed to parse config at {}", path.display()))
}

#[derive(Args, Debug)]
struct InitArgs {
    /// Overwrite the config file if it already exists.
    #[arg(long)]
    force: bool,
}

impl InitArgs {
    pub async fn run(&self, config_path: &Path) -> anyhow::Result<()> {
        if config_path.exists() && !self.force {
            bail!(
                "config already exists at {} (use --force to overwrite)",
                config_path.display()
            );
        }

        let config = Config::default();
        let toml_str =
            toml::to_string_pretty(&config).context("failed to serialize default config")?;

        std::fs::write(config_path, toml_str)
            .with_context(|| format!("failed to write config to {}", config_path.display()))?;

        println!("wrote default config to {}", config_path.display());
        Ok(())
    }
}

#[derive(Args, Debug)]
struct ShowArgs;

impl ShowArgs {
    pub async fn run(&self, config: Config) -> anyhow::Result<()> {
        println!("{}", Show::build(config).await?);
        Ok(())
    }
}

#[derive(Args, Debug)]
struct TxArgs {
    /// Number of rounds in the Up phase before Down is forced (total run
    /// length is double this).
    #[arg(long, default_value_t = 20)]
    steps: u32,
}

impl TxArgs {
    pub async fn run(&self, config: Config) -> anyhow::Result<()> {
        let mut strategy = StepStrategy::new(&config.accounts, self.steps);
        tx::run(config, self.steps.saturating_mul(2), &mut strategy).await
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    dotenvy::dotenv().ok();
    Cli::parse().run().await
}
