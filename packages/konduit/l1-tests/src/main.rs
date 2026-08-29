use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use clap::{Args, Parser, Subcommand};
mod compact_toml;

mod hashing;
pub use hashing::hash32;

mod time;
pub use time::now;

pub mod account;
pub mod adaptor;
pub mod cardano;
pub mod wait;
pub mod wallet;

pub mod scenario;

mod show;
use show::Show;

mod runner;
pub use runner::Runner;

mod play;

mod config;
use config::Config;

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
    /// "Play" a scenario
    Play(PlayArgs),
}

impl Cli {
    /// Dispatches to whichever subcommand was selected. `init` only needs
    /// the config *path* (it's writing a fresh one); everything else needs
    /// the actual loaded `Config`, owned, so we load it inline and hand
    /// ownership down.
    pub async fn run(self) -> anyhow::Result<()> {
        match self.command {
            Command::Init(args) => args.run(&self.config).await,
            Command::Show(args) => args.run(Config::load(&self.config)?).await,
            Command::Play(args) => args.run(Config::load(&self.config)?).await,
        }
    }
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
        Config::default().write(config_path)?;
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
struct PlayArgs {
    /// Execution offset: skip this many leading entries in
    /// `config.scenario.txs` (their `Wait`s included) - for resuming a
    /// run against a chain that's already partway through the scenario,
    /// not for skipping time.
    #[arg(long, default_value_t = 0)]
    from: usize,
}

impl PlayArgs {
    pub async fn run(&self, config: Config) -> anyhow::Result<()> {
        let toml_str = compact_toml::pretty_compact_with(&config, |path| {
            matches!(path, [a, b, ..] if a == "scenario" && b == "txs") && path.len() > 2
        })
        .context("failed to serialize default config")?;
        play::run(config, self.from).await
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    dotenvy::dotenv().ok();
    Cli::parse().run().await
}
