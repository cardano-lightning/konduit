use clap::Parser;
use konduit_hammer::cli::Cli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    if let Some(cmd) = cli.command {
        cmd.run(cli.config).await
    } else {
        Ok(())
    }
}
