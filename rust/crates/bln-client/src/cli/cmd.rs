use crate::types::Payee;
use std::time::Duration;

/// Subcommands for the different API endpoints.
#[derive(Debug, clap::Subcommand)]
pub enum Cmd {
    /// Get a quote for paying an invoice.
    Quote {
        /// The amount in millisatoshis for the quote.
        #[arg(long)]
        amount_msat: u64,

        /// The 33-byte public key of the payee in hex format.
        #[arg(long)]
        payee: Payee,
    },
    /// Pay based on a previous quote.
    Pay {
        /// Max routing fee (msat) that the adaptor is willing to pay.
        #[arg(long)]
        fee_limit: u64,

        /// The relative timeout used to calculate a CLTV limit.
        #[arg(long, value_parser = humantime::parse_duration)]
        timeout: Duration,

        /// The BOLT11 invoice to pay.
        #[arg(long)]
        invoice: String,
    },
}
