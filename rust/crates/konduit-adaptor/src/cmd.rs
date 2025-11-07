use crate::info;
use clap::{Args, Parser};

pub mod metavar;

#[derive(Parser, Debug, Clone)]
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

#[derive(Debug, Clone, Args)]
pub struct Host {
    #[arg(long, env = crate::env::HOST, default_value = "127.0.0.1")]
    pub host: String,
    #[arg(long, env = crate::env::PORT, default_value = "4444")]
    pub port: u16,
}
