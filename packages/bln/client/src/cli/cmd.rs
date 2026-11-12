/// Subcommands for the different API endpoints.
#[derive(Debug, clap::Subcommand)]
pub enum Cmd {
    /// Check backend connectivity and node synchronization status.
    Health,
    /// Create a fixed-amount invoice and print its BOLT11 string.
    Invoice {
        /// Invoice amount in millisatoshis (1,000,000 msat = 1,000 sats).
        #[arg(long)]
        amount_msat: u64,

        /// Human-readable invoice description.
        #[arg(long)]
        description: String,
    },
    /// Quote an invoice and print the amount and maximum total debit.
    Quote {
        /// BOLT11 invoice to quote.
        #[arg(long)]
        invoice: String,
    },
    /// Requote and commit only if the fee still matches the explicitly approved quote.
    Commit {
        /// BOLT11 invoice that was quoted.
        #[arg(long)]
        invoice: String,

        /// Fee in millisatoshis shown by `quote`; commit aborts if it has changed.
        #[arg(long)]
        quoted_fee_msat: u64,

        /// Timeout in seconds shown by `quote`; commit aborts if it has changed.
        #[arg(long)]
        quoted_timeout_seconds: u64,
    },
}
