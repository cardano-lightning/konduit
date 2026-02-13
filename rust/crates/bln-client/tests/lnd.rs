use bln_client::{
    Api,
    lnd::{self, Macaroon},
    types::RevealRequest,
};
use dotenvy::dotenv;
use std::{env, time::Duration};

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
#[ignore]
async fn test_v1_getinfo_success() {
    let config = setup_config();
    let client = lnd::Client::try_from(config).unwrap();

    let result = client.v1_getinfo().await;
    assert!(result.is_ok(), "Expected success, got: {:?}", result.err());
}

/// This is a bad test since it relies on a specific node. and history
#[tokio::test]
#[ignore]
async fn test_reveal() {
    let config = setup_config();
    let client = lnd::Client::try_from(config).unwrap();
    let lock = <[u8; 32]>::try_from(
        hex::decode("af1d3781312baa93c7687305df6ea6f01927d7752a5281b37f7d5acaeedaab0c").unwrap(),
    )
    .unwrap();
    let result = client.reveal(RevealRequest { lock }).await;
    assert!(result.is_ok(), "Expected success, got: {:?}", result.err());
    assert_eq!(
        hex::encode(result.unwrap().secret.unwrap()),
        "ec981cc41b90059035a9fa1e795115568b95b31cd3960089f77153ab57458ece",
        "bad result"
    );
}
