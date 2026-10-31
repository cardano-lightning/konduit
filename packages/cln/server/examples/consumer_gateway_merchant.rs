//! Generates three mutually-consistent config files for a local,
//! three-process demo: merchant <- gateway <- consumer. Run:
//!
//!   cargo run -p cln-server --example consumer_gateway_merchant --features "server client"
//!
//! then save each printed block to its own file and run:
//!   cln-server run --config pay_example/merchant.toml
//!   cln-server run --config pay_example/gateway.toml
//!   cln-consumer pay --config pay_example/consumer.toml --merchant http://127.0.0.1:7653 --amount 1000

use cln_server::{
    channel,
    standalone::{Config, client, config::ServerConfig, inbound, inbounds},
};
use konduit_data::{SigningKey, Tag};

// Fixed, obviously-fake signing keys — distinct per party so the demo
// is internally consistent, never meant for real use.
const CONSUMER_KEY: [u8; 32] = [1; 32];
const GATEWAY_KEY: [u8; 32] = [2; 32];
const MERCHANT_KEY: [u8; 32] = [3; 32];

const CONSUMER_GATEWAY_TAG: &[u8] = b"consumer-gateway";
const GATEWAY_MERCHANT_TAG: &[u8] = b"gateway-merchant";

// Not yet plumbed in.
const BACKING: (u64, u64) = (100_000_000, 0);

fn main() -> anyhow::Result<()> {
    let gateway_url =
        std::env::var("CLN_GATEWAY_URL").unwrap_or_else(|_| "127.0.0.1:7652".to_string());
    let merchant_url =
        std::env::var("CLN_MERCHANT_URL").unwrap_or_else(|_| "127.0.0.1:7653".to_string());
    // Reuse Konduit's own adaptor fee (KONDUIT_FEE) as the CLN routing fee,
    // rather than silently falling back to Params::default()'s test
    // placeholder — keeps the demo's quoted fee consistent with `/info`.
    let fee: u64 = std::env::var("KONDUIT_FEE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| cln_server::ctx::Params::default().fee);
    let consumer_signer = SigningKey::from(CONSUMER_KEY);
    let gateway_signer = SigningKey::from(GATEWAY_KEY);
    let merchant_signer = SigningKey::from(MERCHANT_KEY);

    let consumer_vkey = consumer_signer.verifying_key();
    let gateway_vkey = gateway_signer.verifying_key();
    let merchant_vkey = merchant_signer.verifying_key();

    let cg_tag = Tag::from(CONSUMER_GATEWAY_TAG.to_vec());
    let gm_tag = Tag::from(GATEWAY_MERCHANT_TAG.to_vec());

    // --- consumer -> gateway channel ---
    let consumer_gateway = channel::Config {
        tag: cg_tag.clone(),
        key: gateway_vkey,
        base_url: format!("http://{gateway_url}"),
        media_type: Default::default(),
    };

    // --- gateway -> merchant channel ---
    let gateway_merchant = channel::Config {
        tag: gm_tag.clone(),
        key: merchant_vkey,
        base_url: format!("http://{merchant_url}"),
        media_type: Default::default(),
    };

    // --- merchant config: no outbound channels (terminal node), one
    //     inbound entry admitting the gateway ---
    let merchant = Config {
        inbound: inbounds::Config {
            inbounds: vec![inbound::Config {
                key: gateway_vkey,
                tag: gm_tag,
                backing: BACKING,
            }],
        },
        outbound: cln_server::Config {
            signer: cln_server::signer::Config { key: MERCHANT_KEY },
            channels: cln_server::channels::Config { channels: vec![] },
            paymes: cln_server::paymes::Config {
                default_timeout: konduit_data::Duration::from_secs(300),
            },
            params: cln_server::ctx::Params {
                fee,
                ..Default::default()
            },
        },
        server: ServerConfig {
            listen: merchant_url,
            sync_interval_secs: 5,
        },
    };

    // --- gateway config: one outbound channel (to merchant), one
    //     inbound entry admitting the consumer ---
    let gateway = Config {
        inbound: inbounds::Config {
            inbounds: vec![inbound::Config {
                key: consumer_vkey,
                tag: cg_tag.clone(),
                backing: BACKING,
            }],
        },
        outbound: cln_server::Config {
            signer: cln_server::signer::Config { key: GATEWAY_KEY },
            channels: cln_server::channels::Config {
                channels: vec![gateway_merchant],
            },
            paymes: cln_server::paymes::Config {
                default_timeout: konduit_data::Duration::from_secs(3600),
            },
            params: cln_server::ctx::Params {
                fee,
                ..Default::default()
            },
        },
        server: ServerConfig {
            listen: gateway_url,
            sync_interval_secs: 5,
        },
    };

    // --- consumer config: one outbound channel (to gateway) ---
    let consumer = client::Config {
        signer: cln_server::signer::Config { key: CONSUMER_KEY },
        gateway: consumer_gateway,
    };

    write_example_configs(&merchant, &gateway, &consumer)?;
    Ok(())
}

fn write_example_configs(
    merchant: &Config,
    gateway: &Config,
    consumer: &client::Config,
) -> anyhow::Result<()> {
    let dir = std::env::var_os("CLN_EXAMPLE_CONFIG_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("pay_example"));
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
