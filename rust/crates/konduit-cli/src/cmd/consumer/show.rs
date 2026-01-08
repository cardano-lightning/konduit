use crate::config::consumer::Config;

/// Show
#[derive(clap::Subcommand)]
pub enum Cmd {
    /// Show config. This is a parsed version of env
    Config,
}

impl Cmd {
    pub(crate) fn run(self, config: &Config) -> anyhow::Result<()> {
        match self {
            Cmd::Config => {
                println!("{:#}", toml::to_string_pretty(config)?);
                Ok(())
            }
        }
    }
}
