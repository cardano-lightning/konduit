#[derive(Debug, Clone, clap::Args)]
pub struct Args {
    #[arg(long, env = crate::env::MIN_SINGLE, default_value_t = 1000)]
    pub min_single: u64,
    #[arg(long, env = crate::env::MIN_TOTAL, default_value_t = 1_000_000)]
    pub min_total: u64,
}
