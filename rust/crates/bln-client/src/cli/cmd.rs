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

#[derive(Debug, Clone)]
pub struct Payee(pub [u8; 33]);

impl std::str::FromStr for Payee {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let vec = hex::decode(s).map_err(|err| err.to_string())?;
        let arr = <[u8; 33]>::try_from(vec).map_err(|_| "Wrong length".to_string())?;
        Ok(Payee(arr))
    }
}

impl AsRef<[u8; 33]> for Payee {
    fn as_ref(&self) -> &[u8; 33] {
        &self.0
    }
}
