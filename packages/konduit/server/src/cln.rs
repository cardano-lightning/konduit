use anyhow::Context;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, clap::Args)]
pub struct ClnArgs {
    /// Optional Konduit CLN configuration containing inbound peers and the local CLN node config.
    #[arg(long, env = "KONDUIT_CLN_CONFIG_PATH")]
    pub config_path: Option<PathBuf>,
}
pub use ClnArgs as Args;
pub type Node = cln_server::Ctx;

#[derive(Debug, Deserialize)]
struct Config {
    #[serde(default = "empty_inbounds")]
    inbound: cln_server::inbounds::Config,
    #[serde(default)]
    outbound: cln_server::Config,
}

fn empty_inbounds() -> cln_server::inbounds::Config {
    cln_server::inbounds::Config { inbounds: vec![] }
}

/// Builds CLN's reusable peer context from Konduit's deployment configuration.
pub fn build(args: Args) -> anyhow::Result<cln_server::Ctx> {
    let config = match args.config_path {
        Some(path) => {
            let raw = std::fs::read_to_string(&path)
                .with_context(|| format!("reading Konduit CLN config at {}", path.display()))?;
            toml::from_str(&raw)
                .with_context(|| format!("parsing Konduit CLN config at {}", path.display()))?
        }
        None => Config {
            inbound: empty_inbounds(),
            outbound: cln_server::Config {
                params: Default::default(),
                signer: cln_server::signer::Config {
                    key: cln_server::random::arr32(),
                },
                channels: cln_server::channels::Config { channels: vec![] },
                paymes: Default::default(),
            },
        },
    };
    cln_server::Ctx::init(config.inbound, config.outbound).map_err(anyhow::Error::from)
}

#[cfg(test)]
mod tests {
    use super::{Args, Config, build};
    use cln_server::wire::{commit, payme, quote};
    use konduit_data::{ChequeBody, Duration, Locked, Signature};

    #[tokio::test]
    async fn local_payme_can_be_quoted_and_committed() {
        let cln = build(Args { config_path: None }).expect("local CLN context initializes");
        let payme = cln
            .payme(payme::Request { amount: 1_000_000 })
            .expect("payme is created");
        let quoted = cln
            .quote_outbound(quote::Request {
                payee: payme.payee,
                amount: payme.amount,
            })
            .await
            .expect("self quote succeeds");
        assert_eq!(quoted.fee, 0);

        let timeout = cln_server::time::now() + quoted.timeout + Duration::from_secs(30);
        let locked = Locked::new(
            ChequeBody::new(0, payme.amount, timeout, payme.lock),
            Signature::from_bytes([0; 64]),
        );
        let committed = cln
            .commit_outbound(commit::Request {
                inbound: locked,
                outbound: None,
            })
            .await
            .expect("local commit reveals the payme secret");
        assert!(committed.secret.is_some());
    }

    #[test]
    fn config_does_not_require_standalone_server_settings() {
        let config: Config = toml::from_str(
            r#"
            [outbound.params]
            fee = 1414
            [outbound.params.hop_timeout]
            secs = 300
            nanos = 0
            [outbound.signer]
            key = "0101010101010101010101010101010101010101010101010101010101010101"
            [outbound.channels]
            channels = []
            [outbound.paymes.default_timeout]
            secs = 300
            nanos = 0
            "#,
        )
        .expect("integration config parses without a standalone server section");
        assert!(config.inbound.inbounds.is_empty());
    }
}
