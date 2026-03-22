use clap::Parser;

use crate::config::Config;

#[derive(Debug, Parser)]
#[command(author, version, about = "Konduit Hammer")]
#[command(arg_required_else_help = true)]
pub struct Cli {
    /// Path to config toml file
    #[arg(long, default_value = "hammer.toml")]
    pub config: String,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, clap::Subcommand)]
pub enum Command {
    /// Generate a config template
    Generate,
    /// Show config
    ShowConfig,
    /// Show health
    Health,
    /// Show current state of tip with respect to adaptors and consumers
    Tip,
    /// Run some actions
    Run,
}

impl Command {
    pub async fn run(self, config: String) -> anyhow::Result<()> {
        match self {
            Command::Generate => Config::example().save(config),
            Command::ShowConfig => {
                let config = Config::load(config)?;
                println!("{}", toml::to_string_pretty(&config).unwrap());
                Ok(())
            }
            Command::Health => {
                let config = Config::load(config)?;
                let pool = config.build().await?;
                pool.health().await?;
                Ok(())
            }
            Command::Tip => {
                let config = Config::load(config)?;
                let pool = config.build().await?;
                pool.tip().await?;
                Ok(())
            }
            Command::Run => {
                todo!()
            }
        }
    }
}
