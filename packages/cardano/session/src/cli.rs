use cardano_connector::CardanoConnector;
use cardano_wallet::Wallet;
use clap::{Parser, Subcommand};

// Only `addressbook` and `cmd` are `pub` - they're the two pieces an
// embedding CLI (e.g. konduit-session's) genuinely needs directly:
// `addressbook::Cmd::run` for the session-free bypass (see `Cmd`'s
// doc comment), and `cmd` for `hydrate`/`persist`/the load/save
// primitives. `wallet`/`refresh`/`tip` are reached only through
// `Cmd::run` below, which is the actual unit of reuse - not the
// modules themselves.
pub mod addressbook;
pub mod cmd;
mod refresh;
mod tip;
mod wallet;

#[derive(Debug, Parser)]
#[command(
    name = "cardano-session",
    about = "Drive a CardanoSession from the command line",
    // FIXME :: version = concat!(env!("CARGO_PKG_VERSION"), " (", env!("GIT_HASH"), ")"),
)]
pub struct Cli {
    /// Path to a session config file.
    #[arg(long, default_value = "cardano-session-config.toml")]
    pub config: std::path::PathBuf,

    /// Force a refresh of the wallet/tip from the connector before
    /// running the command. Default is to use whatever's cached.
    #[arg(long)]
    pub refresh: bool,

    #[command(subcommand)]
    pub cmd: Cmd,
}

/// Public, and so is `run` below, so an embedding CLI (e.g.
/// konduit-session's) can nest this enum as one of its own subcommands
/// and dispatch it against a `Session` it already built - `Init` and
/// `Addressbook` excepted, since those are inherently "before a
/// session exists" concerns tied to config, and `run_with` below
/// already handles both without an embedder needing to special-case
/// them itself.
#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// Write a default config to `--config`.
    Init {
        #[arg(long)]
        force: bool,
    },
    /// Wallet operations, passed straight through to `cardano_wallet::Cmd`.
    #[command(subcommand)]
    Wallet(cardano_wallet::Cmd),
    /// Label <-> address bookkeeping. Runs without a session - pure
    /// local file editing.
    #[command(subcommand)]
    Addressbook(addressbook::Cmd),
    /// Sync tip data from the chain - nothing does this implicitly.
    #[command(subcommand)]
    Refresh(refresh::Cmd),
    /// Tracking and reading tip data.
    #[command(subcommand)]
    Tip(tip::Cmd),
}

impl Cmd {
    /// Dispatches `self` against an already-built, already-hydrated
    /// `session` - the actual reusable unit, callable equally from
    /// this crate's own `Cli::run_with` and from anything embedding this
    /// enum. `Init`/`Addressbook` can't be handled here (there's no
    /// session to hand them, and `Addressbook` mustn't force one to be
    /// built) - `run_with` intercepts both before this is ever called.
    pub async fn run<C: CardanoConnector, W: Wallet>(
        &self,
        session: &mut crate::Session<C, W>,
    ) -> anyhow::Result<()> {
        match self {
            Cmd::Init { .. } | Cmd::Addressbook(_) => anyhow::bail!(
                "`{self:?}` needs to be handled by the caller before a session is built"
            ),
            Cmd::Wallet(wallet_cmd) => wallet::run(session, wallet_cmd).await,
            Cmd::Refresh(refresh_cmd) => refresh_cmd.run(session).await,
            Cmd::Tip(tip_cmd) => tip_cmd.run(session).await,
        }
    }
}

impl Cli {
    // TODO: hardwired to Session<Blockfrost>. A different connector would
    // need `connector::Config` to be enum-dispatched here.
    pub async fn run(self) -> anyhow::Result<()> {
        if let Cmd::Init { force } = &self.cmd {
            return cmd::init(&self.config, *force);
        }

        let config = cmd::load_config(&self.config)?;
        Self::run_with(config, self.refresh, self.cmd).await
    }

    /// The reusable unit for embedding: everything `run` does except
    /// owning a config *path* - a caller that already has its own
    /// top-level `--config`/`--refresh` (this crate's own `run`, or an
    /// embedding CLI like konduit-session's) hands down an
    /// already-loaded `Config` and parsed options instead of a path to
    /// re-read, so state is never duplicated across layers. `Init` isn't
    /// representable here - it's about writing a config file at a path,
    /// not running against a loaded one - so it's rejected up front
    /// rather than silently building a session first.
    pub async fn run_with(config: crate::Config, refresh: bool, cmd: Cmd) -> anyhow::Result<()> {
        if let Cmd::Init { .. } = &cmd {
            anyhow::bail!(
                "`init` needs a config path, not a loaded config - not meaningful when embedded"
            );
        }

        // Bypasses session construction - addressbook editing never needs a
        // connector, so this is decided from `config` alone.
        if let Cmd::Addressbook(addressbook_cmd) = &cmd {
            let mut book = cmd::load_addressbook(&config.addressbook_path)?;
            let result = addressbook_cmd.run(&mut book);
            if let Err(e) = cmd::save_addressbook(&book, &config.addressbook_path) {
                eprintln!("warning: failed to save addressbook: {e}");
            }
            return result;
        }

        // Taken before `Session::init` consumes `config`.
        let tip_cache_path = config.tip_cache_path.clone();
        let addressbook_path = config.addressbook_path.clone();
        let mut session = crate::Session::init(config).await?;

        cmd::hydrate(&mut session, &tip_cache_path, &addressbook_path, refresh).await?;

        let result = cmd.run(&mut session).await;

        cmd::persist(
            session.tip(),
            session.addressbook(),
            &tip_cache_path,
            &addressbook_path,
        );

        result
    }
}
