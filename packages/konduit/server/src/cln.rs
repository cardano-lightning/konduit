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
pub type Node = cln_server::standalone::Ctx;

#[derive(Debug, Deserialize)]
struct Config {
    #[serde(default = "empty_inbounds")]
    inbound: cln_server::standalone::inbounds::Config,
    #[serde(default)]
    outbound: cln_server::Config,
}

fn empty_inbounds() -> cln_server::standalone::inbounds::Config {
    cln_server::standalone::inbounds::Config { inbounds: vec![] }
}

/// Builds CLN's reusable peer context from Konduit's deployment configuration.
///
/// `fee` is Konduit's own adaptor fee (`CommonArgs.fee` / `KONDUIT_FEE`) and
/// is only used as the CLN routing fee default when no `config_path` is
/// given — the only fee knob a deployment actually exposes today, reused
/// here rather than left silently defaulted to a test placeholder.
///
/// Accepting inbound CLN peer commits here is only safe because
/// `server::Data::cln_commit` (and `cln_quote`/`cln_sync`) verify the
/// Konduit channel backing the peer's keytag is actually funded on-chain
/// before ever reaching this context — `Ctx` itself has no way to check
/// that.
pub fn build(args: Args, fee: u64) -> anyhow::Result<cln_server::standalone::Ctx> {
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
                params: cln_server::ctx::Params {
                    fee,
                    ..Default::default()
                },
                signer: cln_server::signer::Config {
                    key: cln_server::random::arr32(),
                },
                channels: cln_server::channels::Config { channels: vec![] },
                paymes: Default::default(),
            },
        },
    };
    cln_server::standalone::Ctx::init(cln_server::standalone::Config {
        // Konduit never runs CLN's own HTTP listener or periodic sync loop
        // (those are driven by konduit-server's own main loop instead), so
        // these settings are always defaulted rather than read from the
        // deployment's config file.
        server: Default::default(),
        inbound: config.inbound,
        outbound: config.outbound,
    })
    .map_err(anyhow::Error::from)
}

#[cfg(test)]
mod tests {
    use super::{Args, Config, build};
    use cln_server::wire::{commit, payme, quote};
    use konduit_data::{ChequeBody, Duration, Locked, Signature};

    #[tokio::test]
    async fn local_payme_can_be_quoted_and_committed() {
        let cln = build(Args { config_path: None }, 0).expect("local CLN context initializes");
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
