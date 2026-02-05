use clap::Parser;

// FIXME
// https://github.com/clap-rs/clap/issues/4279
// Clap's derive `Args` uses the struct name verbatim.
// There doesn't seem to be any attribute declaration to get around this.
// I think we still want to use derive since its fewer LoC.
// However we now do this weird dance of `pub struct MyServiceArgs` and `pub use
// args::MyServiceArgs as Args`.

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// ## Common
    #[command(flatten)]
    pub common: crate::common::Args,

    /// ## Cardano Connect
    #[command(flatten)]
    pub cardano: crate::cardano::Args,

    /// ## Server
    #[command(flatten)]
    pub server: crate::server::Args,

    /// ## Admin
    #[command(flatten)]
    pub admin: crate::admin::Args,

    /// ## DB
    #[command(flatten)]
    pub db: crate::db::Args,

    /// ## BLN
    #[command(flatten)]
    pub bln: bln_client::cli::Args,

    /// ## FX
    #[command(flatten)]
    pub fx: fx_client::cli::Args,
}
