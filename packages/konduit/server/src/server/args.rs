#[derive(Debug, Clone, clap::Args)]
pub struct ServerArgs {
    #[arg(long, env = crate::env::SERVER_HOST, default_value = "127.0.0.1")]
    pub host: String,
    #[arg(long, env = crate::env::SERVER_PORT, default_value = "5663")]
    pub port: u16,
}
