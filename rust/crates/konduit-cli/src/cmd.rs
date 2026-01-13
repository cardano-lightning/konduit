mod adaptor;
mod admin;
mod consumer;
mod parsers;

/// A utility for constructing and driving Konduit's stages
#[derive(clap::Parser)]
#[clap(version = env!("CARGO_PKG_VERSION"), about, long_about = None)]
pub(crate) enum Cmd {
    #[clap(subcommand)]
    Adaptor(adaptor::Cmd),

    #[clap(subcommand)]
    Admin(admin::Cmd),

    #[clap(subcommand)]
    Consumer(consumer::Cmd),
}

impl Cmd {
    pub(crate) fn run(self) -> anyhow::Result<()> {
        match self {
            Self::Adaptor(cmd) => cmd.run(),
            Self::Admin(cmd) => cmd.run(),
            Self::Consumer(cmd) => cmd.run(),
        }
    }
}
