//! Generates the mutually-consistent config files for the local demo:
//! merchant <- gateway <- consumer. The two server roles are
//! `konduit-server` processes, which read their rail config from
//! `KONDUIT_CLN_CONFIG_PATH` and their on-chain identity from
//! `.env.<role>`; the consumer is the `cln-client` binary. The nix
//! dev-env runs this as `generate-cln-demo-configs`:
//!
//!   cargo run -p cln-rail --example consumer_gateway_merchant --features client
//!
//! then pays with:
//!   cln-client --config pay_example/consumer.toml pay \
//!     --merchant http://127.0.0.1:7653 --amount 1000

use cln_rail::{channel, client, commits, config::Config};
use konduit_data::{SigningKey, Tag};

// Fixed, obviously-fake signing keys — distinct per party so the demo
// is internally consistent, never meant for real use.
const CONSUMER_KEY: [u8; 32] = [1; 32];
const GATEWAY_KEY: [u8; 32] = [2; 32];
const MERCHANT_KEY: [u8; 32] = [3; 32];

const CONSUMER_GATEWAY_TAG: &[u8] = b"consumer-gateway";
const GATEWAY_MERCHANT_TAG: &[u8] = b"gateway-merchant";

fn main() -> anyhow::Result<()> {
    let gateway_url =
        std::env::var("CLN_GATEWAY_URL").unwrap_or_else(|_| "127.0.0.1:7652".to_string());
    let merchant_url =
        std::env::var("CLN_MERCHANT_URL").unwrap_or_else(|_| "127.0.0.1:7653".to_string());
    // No fee here: konduit's own fee is konduit's (KONDUIT_FEE, read by
    // konduit-server), and the rail charges nothing of its own.
    let gateway_signer = SigningKey::from(GATEWAY_KEY);
    let merchant_signer = SigningKey::from(MERCHANT_KEY);

    let gateway_vkey = gateway_signer.verifying_key();
    let merchant_vkey = merchant_signer.verifying_key();

    let cg_tag = Tag::from(CONSUMER_GATEWAY_TAG.to_vec());
    let gm_tag = Tag::from(GATEWAY_MERCHANT_TAG.to_vec());

    // Where each peer is reached. A known key is a peer that could be
    // paid; the channel entries below are the ones actually held.
    let gateway_known = channel::KnownKey {
        key: gateway_vkey,
        base_url: format!("http://{gateway_url}"),
        media_type: Default::default(),
    };
    let merchant_known = channel::KnownKey {
        key: merchant_vkey,
        base_url: format!("http://{merchant_url}"),
        media_type: Default::default(),
    };

    // --- merchant config: no outbound peers, since it is the terminal
    //     payee and pays no one onward ---
    let merchant = Config {
        signer: cln_rail::signer::Config { key: MERCHANT_KEY },
        outbound: cln_rail::channels::Config {
            known_keys: vec![],
            channels: vec![],
        },
        known_commits: commits_config("merchant"),
    };

    // --- gateway config: one outbound peer, the merchant ---
    let gateway = Config {
        signer: cln_rail::signer::Config { key: GATEWAY_KEY },
        outbound: cln_rail::channels::Config {
            known_keys: vec![merchant_known],
            channels: vec![channel::Entry {
                key: merchant_vkey,
                tag: gm_tag.clone(),
            }],
        },
        known_commits: commits_config("gateway"),
    };

    // --- consumer config: one outbound channel (to gateway) ---
    let consumer = client::Config {
        signer: cln_rail::signer::Config { key: CONSUMER_KEY },
        peer: gateway_known,
        tag: cg_tag.clone(),
    };

    write_example_configs(&merchant, &gateway, &consumer)?;
    write_wallet_envs()?;
    Ok(())
}

/// Where a role's rail keeps what it has committed. Beside its config,
/// so a demo rerun that keeps the config dir keeps the commitments its
/// servers are still answering for.
fn commits_config(role: &str) -> commits::Config {
    commits::Config {
        path: Some(config_dir().join(format!("commits.{role}.json"))),
    }
}

fn config_dir() -> std::path::PathBuf {
    std::env::var_os("CLN_EXAMPLE_CONFIG_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("pay_example"))
}

fn write_example_configs(
    merchant: &Config,
    gateway: &Config,
    consumer: &client::Config,
) -> anyhow::Result<()> {
    let dir = config_dir();
    std::fs::create_dir_all(&dir)?;

    for (name, toml_str) in [
        ("merchant", toml::to_string_pretty(merchant)?),
        ("gateway", toml::to_string_pretty(gateway)?),
        ("consumer", toml::to_string_pretty(consumer)?),
    ] {
        let path = dir.join(format!("{name}.toml"));
        std::fs::write(&path, toml_str)?;
        println!("wrote {}", path.display());
    }

    Ok(())
}

/// Writes one `.env.<name>` file per party (konduit-cli's own dotenv
/// convention — see `DefaultPath`), carrying the same signing key as its CLN
/// TOML config above. `konduit-cli` reads `KONDUIT_WALLET`, and the
/// konduit-server processes read `KONDUIT_SIGNING_KEY` — both set to the
/// identical hex key here so that whoever opens an on-chain Konduit channel
/// to a party (consumer -> gateway, gateway -> merchant) signs with exactly
/// the key that party's CLN context uses to recognize and sign commits for
/// that channel's keytag.
///
/// Never overwrites a file that's already there: these are meant to be
/// stable across reruns (and editable by hand), unlike the TOML configs
/// above which are regenerated every time.
fn write_wallet_envs() -> anyhow::Result<()> {
    let dir = config_dir();
    std::fs::create_dir_all(&dir)?;

    for (name, key) in [
        ("consumer", CONSUMER_KEY),
        ("gateway", GATEWAY_KEY),
        ("merchant", MERCHANT_KEY),
    ] {
        let path = dir.join(format!(".env.{name}"));
        let mut opts = std::fs::OpenOptions::new();
        opts.write(true).create_new(true);
        match opts.open(&path) {
            Ok(mut file) => {
                let hex_key = hex::encode(key);
                use std::io::Write;
                writeln!(file, "KONDUIT_WALLET={hex_key}")?;
                writeln!(file, "KONDUIT_SIGNING_KEY={hex_key}")?;
                println!("wrote {}", path.display());
            }
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                println!("kept existing {}", path.display());
            }
            Err(err) => return Err(err.into()),
        }
    }

    Ok(())
}
