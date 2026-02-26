use crate::core::{SigningKey, SquashStatus, Tag};
use clap::Parser;
use std::{io, io::Write};

#[derive(Parser)]
#[command(
    author,
    version,
    about = "Konduit CLI - Factorized manual interaction tool"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// URL of the Konduit server
    #[arg(
        long,
        env = "KONDUIT_SERVER_URL",
        default_value = "http://127.0.0.1:5663"
    )]
    pub server_url: String,

    /// Hex encoded signing key
    #[arg(long, env = "KONDUIT_SIGNING_KEY")]
    pub signing_key: SigningKey,

    /// Hex encoded Tag. Required.
    #[arg(long, env = "KONDUIT_TAG")]
    pub tag: Tag,

    /// Optional LND REST URL
    #[arg(long, env = "LND_BASE_URL")]
    pub lnd_url: Option<String>,

    /// Optional LND Macaroon (Hex)
    #[arg(long, env = "LND_MACAROON")]
    pub lnd_macaroon: Option<String>,

    /// Skip confirmation prompts
    #[arg(short, long)]
    pub yes: bool,
}

#[derive(clap::Subcommand)]
pub enum Commands {
    /// Show info about the server
    Info,
    /// Create an invoice on a local LND node
    AddInvoice { amount_msat: u64, memo: String },
    /// Get a quote for a lightning invoice
    Quote { invoice: String },
    /// Full workflow: Get quote -> Pay -> Squash
    Pay { invoice: String },
    /// Manually squash using the latest state
    Squash,
}

pub fn confirm(prompt: &str) -> anyhow::Result<bool> {
    eprint!("\n{} [y/N] ", prompt);

    io::stderr().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    if input.trim().is_empty() || input.trim().to_lowercase() == "n" {
        return Ok(false);
    }

    if input.trim().to_lowercase() == "y" {
        return Ok(true);
    }

    confirm(prompt)
}

pub fn prompt_if_incomplete(st: &SquashStatus, auto_confirm: bool) -> anyhow::Result<bool> {
    if !auto_confirm && matches!(st, SquashStatus::Incomplete { .. }) {
        confirm("Verify proposal and execute squash?")
    } else {
        Ok(auto_confirm)
    }
}
