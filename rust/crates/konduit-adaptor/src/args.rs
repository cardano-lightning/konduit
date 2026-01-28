use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// ## Common
    #[command(flatten)]
    pub common: crate::common::CommonArgs,

    /// ## Server
    #[command(flatten)]
    pub server: crate::server::ServerArgs,

    /// ## Admin
    #[command(flatten)]
    pub admin: crate::admin::AdminArgs,

    /// ## DB
    #[command(flatten)]
    pub db: crate::db::DbArgs,

    /// ## BLN
    #[command(flatten)]
    pub bln: crate::bln::BlnArgs,

    /// ## FX
    #[command(flatten)]
    pub fx: crate::fx::FxArgs,
}
