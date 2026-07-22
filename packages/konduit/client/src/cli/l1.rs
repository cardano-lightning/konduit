use std::{path::Path, sync::Arc};

use cardano_sdk::SigningKey;
use clap::Subcommand;

use crate::{Cache, Cardano, Config, L1, keys::Embedded, l1::Directives};

#[derive(Debug, Subcommand)]
pub enum Cmd {
    Pull,
    Show,
    Submit,
}

impl Cmd {
    pub async fn run(&self, config_path: &Path, cache_path: &Path) -> anyhow::Result<()> {
        let cfg = Config::load(config_path)?;
        let cache = Cache::load(cache_path)?;

        let cardano = Arc::new(Cardano::new(cfg.cardano()).await?);

        let wallet_signer = Arc::new(SigningKey::from(
            cfg.keys().wallet()?.expect("Signer required"),
        ));
        let wallet = Arc::new(Embedded::new(cardano.clone(), wallet_signer, None));

        let signer = Arc::new(SigningKey::from(
            cfg.keys().signer()?.expect("Signer required"),
        ));

        /// TODO. FIXME.
        let directives = Directives::default();

        let mut l1 = L1::with_cache(
            cardano,
            signer,
            wallet,
            cfg.l1().clone(),
            cache.l1().to_owned(),
            directives,
        );
        match self {
            Cmd::Pull => {
                l1.pull_all().await?;
                Ok(())
            }
            Cmd::Show => todo!(),
            Cmd::Submit => todo!(),
        }
    }
}
