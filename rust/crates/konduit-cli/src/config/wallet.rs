use crate::config::signing_key::SigningKey;

/// Wallet signing key
#[derive(Debug, Clone, clap::Args)]
pub struct Wallet {
    /// Hex encoded wallet signing key (omitting this triggers generation/env fallback)
    #[arg(long)]
    pub wallet: Option<SigningKey>,
}
