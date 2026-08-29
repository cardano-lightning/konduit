use cardano_sdk::Input;
use clap::{Parser, Subcommand};
use konduit_tx2::Channel;

mod cmd;

/// A thin wrapper around cardano-session
/// with helpers for managing konduit specific tasks.
#[derive(Debug, Parser)]
#[command(
    name = "konduit-session",
    // FIXME :: version = concat!(env!("CARGO_PKG_VERSION"), " (", env!("GIT_HASH"), ")"),
)]
pub struct Cli {
    /// Path to a session config file.
    #[arg(long, default_value = "konduit-session-config.toml")]
    pub config: std::path::PathBuf,

    /// Force a refresh of the underlying cardano session.
    #[arg(long)]
    pub refresh: bool,

    #[command(subcommand)]
    pub cmd: Cmd,
}

#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// Write a default config to `--config`.
    Init {
        #[arg(long)]
        force: bool,
    },
    /// Upload konduit script
    Upload,
    /// Teardown konduit script
    Teardown,
    /// List open channels at the current tip.
    Channels,
    /// Pass through to the cardano session.
    #[command(subcommand)]
    Cardano(cardano_session::cli::Cmd),
}

impl Cli {
    pub async fn run(self) -> anyhow::Result<()> {
        let Cli {
            config,
            refresh,
            cmd,
        } = self;

        if let Cmd::Init { force } = &cmd {
            return cmd::init(&config, *force);
        }

        let config = cmd::load_config(&config)?;
        Self::run_with(config.session, refresh, cmd).await
    }

    pub async fn run_with(
        config: cardano_session::Config,
        refresh: bool,
        cmd: Cmd,
    ) -> anyhow::Result<()> {
        if let Cmd::Init { .. } = &cmd {
            anyhow::bail!(
                "`init` needs a config path, not a loaded config - not meaningful when embedded"
            );
        }

        let cmd = match cmd {
            Cmd::Cardano(inner) => {
                return cardano_session::cli::Cli::run_with(config, refresh, inner).await;
            }
            other => other,
        };

        let tip_cache_path = config.tip_cache_path.clone();
        let addressbook_path = config.addressbook_path.clone();
        let mut cardano = cardano_session::Session::init(config).await?;

        cardano_session::cli::cmd::hydrate(
            &mut cardano,
            &tip_cache_path,
            &addressbook_path,
            refresh,
        )
        .await?;

        let mut session = crate::Session::new(cardano)?;

        let result = match &cmd {
            Cmd::Init { .. } | Cmd::Cardano(_) => unreachable!("handled above"),
            Cmd::Upload => {
                let id = session.upload().await?;
                print_json(&serde_json::json!({ "id": id.to_string() }))
            }
            Cmd::Teardown => {
                let id = session.teardown().await?;
                print_json(&serde_json::json!({ "id": id.to_string() }))
            }
            Cmd::Channels => {
                let channels: Vec<(Input, Channel)> = session.channels().into_iter().collect();
                print_json(&channels)
            }
        };

        cardano_session::cli::cmd::persist(
            session.tip(),
            session.addressbook(),
            &tip_cache_path,
            &addressbook_path,
        );
        result
    }
}

// Every subcommand prints its result through here, so stdout is always
// JSON and safe to pipe into downstream tooling.
fn print_json(value: &impl serde::Serialize) -> anyhow::Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}
