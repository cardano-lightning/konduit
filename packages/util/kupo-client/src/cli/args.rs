#[derive(Debug, Clone, clap::Parser)]
#[clap(
    version,
    about = "Query a running Kupo server and print matches as JSON.",
    long_about = None
)]
pub struct Args {
    /// Hostname or IP of the Kupo server.
    #[arg(long, env = "KUPO_HOST", default_value = "127.0.0.1")]
    pub host: String,

    /// TCP port of the Kupo server.
    #[arg(long, env = "KUPO_PORT", default_value_t = 1442)]
    pub port: u16,

    /// Optional match pattern. If omitted, matches are fetched across all
    /// configured kupo patterns (`GET /matches`). If given, only the
    /// pattern at `GET /matches/{pattern}` is queried. The string is used
    /// verbatim as a Kupo pattern.
    #[arg(long, env = "KUPO_PATTERN")]
    pub pattern: Option<String>,

    /// Maximum number of matches to print.
    #[arg(long, env = "KUPO_LIMIT", default_value_t = 10)]
    pub limit: usize,

    /// Whether to resolve and inline `datum` / `script` references in the
    /// printed matches.
    #[arg(long, env = "KUPO_RESOLVE_HASHES", default_value_t = false)]
    pub resolve_hashes: bool,
}
