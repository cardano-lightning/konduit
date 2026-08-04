use clap::Parser;
mod cmd;

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    Text,
    Yaml,
    Json,
}

#[derive(Debug, clap::Parser)]
#[clap(
    version,
    about = "Konduit channel-state indexer: walks Kupo matches into the local SQLite DB.",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, clap::Subcommand)]
enum Cmd {
    Index(cmd::index::Args),
    Show(cmd::show::Args),
}

#[cfg(feature = "cli")]
fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Index(args) => cmd::index::run(args),
        Cmd::Show(args) => cmd::show::run(args),
    }
}
