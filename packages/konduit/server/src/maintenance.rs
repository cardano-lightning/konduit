use clap::{Parser, Subcommand};
use konduit_server::db::Db;

#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Args {
    #[arg(long, env = "KONDUIT_DB_PATH", default_value = "konduit.db")]
    db_path: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Remove one known-failed Lightning payment from the channel and authorization tables.
    CancelFailedPayment {
        #[arg(long)]
        payment_hash: String,

        /// Confirm that LND reported this payment as FAILED with no preimage.
        #[arg(long)]
        confirm_payment_failed: bool,
    },
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    match args.command {
        Command::CancelFailedPayment {
            payment_hash,
            confirm_payment_failed,
        } => {
            anyhow::ensure!(
                confirm_payment_failed,
                "refusing cleanup without --confirm-payment-failed"
            );
            let payment_hash = hex::decode(payment_hash)?
                .try_into()
                .map_err(|_| anyhow::anyhow!("payment hash must be exactly 32 bytes"))?;
            let db = Db::open(&args.db_path)?;
            let (keytag, index, amount) = db.cancel_failed_payment(&payment_hash)?;
            println!(
                "cancelled failed payment for channel {keytag}, cheque index {index}, amount {amount}"
            );
        }
    }
    Ok(())
}
