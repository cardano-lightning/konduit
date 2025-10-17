mod open;

/// A utility for constructing and driving Konduit's stages
#[derive(clap::Parser)]
#[clap(version = env!("CARGO_PKG_VERSION"), about, long_about = None)]
#[clap(propagate_version = true)]
pub(crate) enum Cmd {
    Open(open::Args),
}

impl Cmd {
    pub(crate) async fn execute(self) -> anyhow::Result<()> {
        let connector = crate::connector::new()?;
        match self {
            Self::Open(args) => open::execute(connector, args).await,
        }
    }
}
