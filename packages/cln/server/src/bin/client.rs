use clap::{Parser, Subcommand};
use cln_server::standalone::{Client, client::Config};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "cln-consumer")]
struct Cli {
    #[arg(short, long, default_value = "cln-consumer-config.toml")]
    config: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Write a default config to file.
    Init {
        #[arg(long)]
        force: bool,
    },
    /// Pay a merchant through the gateway.
    Pay {
        #[arg(long)]
        merchant: String,
        #[arg(long)]
        amount: u64,
    },
}

fn init_config(path: &PathBuf, force: bool) -> anyhow::Result<()> {
    let toml = toml::to_string_pretty(&Config::default())?;
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true);
    if force {
        opts.create(true).truncate(true);
    } else {
        opts.create_new(true);
    }
    std::io::Write::write_all(
        &mut opts
            .open(path)
            .map_err(|e| anyhow::anyhow!("opening {} (use --force): {e}", path.display()))?,
        toml.as_bytes(),
    )?;
    log::info!("wrote default config to {}", path.display());
    Ok(())
}

async fn pay(config: PathBuf, merchant: String, amount: u64) -> anyhow::Result<()> {
    let config: Config = toml::from_str(&std::fs::read_to_string(&config)?)?;
    let secret = Client::init(config).pay(&merchant, amount).await?;
    println!("paid. secret: {secret:?}");
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let cli = Cli::parse();
    match cli.command {
        Command::Init { force } => init_config(&cli.config, force),
        Command::Pay { merchant, amount } => pay(cli.config, merchant, amount).await,
    }
}
