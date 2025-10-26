use clap::{Args, Parser};

use crate::info;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cmd {
    /// ## Host
    #[command(flatten)]
    pub host: Host,

    #[command(flatten)]
    pub info: info::Info,

    // /// ## DB
    #[command(flatten)]
    pub db: crate::db::DbArgs,

    // /// ## BLN
    #[command(flatten)]
    pub bln: crate::bln::BlnArgs,

    // /// ## FX
    #[command(flatten)]
    pub fx: crate::fx::FxArgs,
}

#[derive(Debug, Args)]
pub struct Host {
    #[arg(long, env = "KONDUIT_HOST", default_value = "127.0.0.1")]
    pub host: String,
    #[arg(long, env = "KONDUIT_PORT", default_value = "4444")]
    pub port: u16,
}
