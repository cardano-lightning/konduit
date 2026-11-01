use anyhow::Context;
use std::path::PathBuf;

#[derive(Debug, Clone, clap::Args)]
pub struct ClnArgs {
    /// Optional Konduit CLN configuration containing inbound peers and the local CLN node config.
    #[arg(long, env = "KONDUIT_CLN_CONFIG_PATH")]
    pub config_path: Option<PathBuf>,
}
pub use ClnArgs as Args;
type Config = cln_rail::Config;

pub fn build(args: Args) -> anyhow::Result<cln_rail::Ctx> {
    let config = match args.config_path {
        Some(path) => {
            let raw = std::fs::read_to_string(&path)
                .with_context(|| format!("reading Konduit CLN config at {}", path.display()))?;
            toml::from_str(&raw)
                .with_context(|| format!("parsing Konduit CLN config at {}", path.display()))?
        }
        None => Config {
            signer: cln_rail::signer::Config {
                key: cln_rail::random::arr32(),
            },
            channels: cln_rail::channels::Config { channels: vec![] },
            paymes: Default::default(),
        },
    };
    cln_rail::Ctx::init(config).map_err(anyhow::Error::from)
}
