use dotenvy::dotenv;
use std::env;
use std::time::Duration;

use bln_client::lnd::{self, Macaroon};

fn setup_config() -> lnd::Config {
    dotenv().ok();

    let base_url = env::var("LND_BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());

    let macaroon_hex = env::var("LND_MACAROON").expect("LND_MACROON must be set (hex string)");
    let macaroon = macaroon_hex
        .parse::<Macaroon>()
        .expect("LND_MACROON must be valid hex");

    let block_time_secs = env::var("BLOCK_TIME")
        .unwrap_or_else(|_| "600".to_string())
        .parse::<u64>()
        .expect("BLOCK_TIME must be a number");

    lnd::Config {
        base_url,
        macaroon,
        block_time: Duration::from_secs(block_time_secs),
        min_cltv: 84,
        tls_certificate: None,
    }
}

#[tokio::test]
async fn test_v1_getinfo_success() {
    let config = setup_config();
    let client = lnd::Client::try_from(config).unwrap();

    let result = client.v1_getinfo().await;

    assert!(result.is_ok(), "Expected success, got: {:?}", result.err());
}
