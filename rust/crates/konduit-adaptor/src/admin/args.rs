use std::time::Duration;

#[derive(Debug, Clone, clap::Args)]
pub struct AdminArgs {
    /// BLN block time. defaults to 600s = 10 mins
    // Prevent name clash with fx_every
    #[arg( long, env = "ADMIN_EVERY", value_parser = humantime::parse_duration, default_value = "10m",)]
    pub admin_every: Duration,
    #[arg(long, env = crate::env::MIN_SINGLE, default_value_t = 1000)]
    pub min_single: u64,
    #[arg(long, env = crate::env::MIN_TOTAL, default_value_t = 1_000_000)]
    pub min_total: u64,
}
