use clap::{Parser, Subcommand};
use cln_server::standalone::{Client, client::Config};
use std::path::PathBuf;
use tracing_subscriber::{EnvFilter, fmt::format::FmtSpan};

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

/// The command's result, as pretty JSON on stdout — separate from the
/// tracing narration on stderr, so stdout stays clean for downstream use.
fn print_result(value: serde_json::Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(&value).expect("result is valid json")
    );
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
    print_result(serde_json::json!({
        "status": "ok",
        "action": "init",
        "path": path.display().to_string(),
    }));
    Ok(())
}

async fn pay(config: PathBuf, merchant: String, amount: u64) -> anyhow::Result<()> {
    let config: Config = toml::from_str(&std::fs::read_to_string(&config)?)?;
    let paid = Client::init(config).pay(&merchant, amount).await?;
    print_result(serde_json::json!({
        "status": "ok",
        "action": "pay",
        "merchant": merchant,
        "amount": amount,
        "payme_amount": paid.payme_amount,
        "committed_amount": paid.committed_amount,
        "secret": hex::encode(paid.secret.as_ref()),
    }));
    Ok(())
}

/// Step-by-step narration of the pay flow (from `#[tracing::instrument]` on
/// `Client::pay` and friends), written to stderr so it never mixes with the
/// JSON result on stdout. Shows by default without needing `RUST_LOG`.
fn init_tracing() {
    // Same reasoning as konduit-server: ANSI codes land raw in
    // process-compose's captured output (cln-demo-consumer runs this
    // binary as one of its processes) unless the writer is a real tty.
    let ansi = std::io::IsTerminal::is_terminal(&std::io::stderr());
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("cln_server=info")),
        )
        .with_span_events(FmtSpan::NEW)
        .without_time()
        .with_target(false)
        .with_writer(std::io::stderr)
        .with_ansi(ansi)
        .compact()
        .init();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();
    let cli = Cli::parse();
    match cli.command {
        Command::Init { force } => init_config(&cli.config, force),
        Command::Pay { merchant, amount } => pay(cli.config, merchant, amount).await,
    }
}
