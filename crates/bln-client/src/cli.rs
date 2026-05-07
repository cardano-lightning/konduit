mod args;
pub use args::ClientArgs as Args;

mod config;
pub use config::Config;

mod cmd;
pub use cmd::*;

use clap::Parser;

/// Top-level CLI arguments for the BLN system.
#[derive(Debug, Parser)]
#[command(name = "bln", about = "Bitcoin Lightning Network Client CLI")]
pub struct BlnArgs {
    #[command(flatten)]
    pub client: Args,

    #[command(subcommand)]
    pub command: Cmd,
}
