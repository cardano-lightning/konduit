//! Live integration tests against a local Cardano testnet.
//!
//! These tests only run when the testnet plumbing env vars are set; see the
//! module docs in `support.rs`. On machines without a testnet they simply
//! report and exit cleanly.

mod support;

use cardano_connector::CardanoConnector;
use cardano_sdk::Address;
use cardano_sdk::address::kind;
use std::time::Duration;

#[tokio::test]
async fn live_node_connects_and_reports_health() {
    let Ok((_env, node)) = support::live_node().await else {
        eprintln!(
            "skipping: testnet env vars ({}, {}, {}) not all set",
            support::ENV_SOCKET,
            support::ENV_NETWORK_ID,
            support::ENV_FAUCET_ADDR,
        );
        return;
    };

    let health = node.health().await.expect("health check");
    assert!(!health.is_empty(), "health string should not be empty");

    let params = node
        .protocol_parameters()
        .await
        .expect("protocol parameters");
    assert!(
        params.base_fee(1) > params.base_fee(0),
        "base_fee(1) should be larger than base_fee(0)"
    );
}

#[tokio::test]
async fn live_node_returns_faucet_utxos() {
    let Ok((env, node)) = support::live_node().await else {
        eprintln!(
            "skipping: testnet env vars ({}, {}, {}) not all set",
            support::ENV_SOCKET,
            support::ENV_NETWORK_ID,
            support::ENV_FAUCET_ADDR,
        );
        return;
    };

    let faucet_address = &env.faucet_address;
    let utxos = node
        .utxos_at(
            &faucet_address.payment(),
            faucet_address.delegation().as_ref(),
        )
        .await
        .expect("faucet UTxO query");

    assert!(
        !utxos.is_empty(),
        "expected at least one UTxO at the faucet address, got none (is the testnet funded?)"
    );
}

#[tokio::test]
async fn live_node_submits_and_waits_for_settlement() {
    let Ok((env, node)) = support::live_node().await else {
        eprintln!(
            "skipping: testnet env vars ({}, {}, {}) not all set",
            support::ENV_SOCKET,
            support::ENV_NETWORK_ID,
            support::ENV_FAUCET_ADDR,
        );
        return;
    };

    let params = node
        .protocol_parameters()
        .await
        .expect("protocol parameters");

    let (recipient_key, recipient_address) = support::fresh_keypair();
    let recipient_any: Address<kind::Any> = recipient_address.clone().into();

    let tx_id = support::transfer_ada(
        &node,
        &env.faucet_signing_key,
        &recipient_any,
        5_000_000,
        &params,
    )
    .await
    .expect("faucet-to-recipient transfer");

    let recipient_input = cardano_sdk::Input::new(tx_id, 0);

    let utxos = support::wait_for_specific_utxo(
        &node,
        &recipient_address.payment(),
        None,
        &recipient_input,
        Duration::from_secs(180),
    )
    .await
    .expect("recipient UTxO should appear on-chain within the deadline");

    assert!(
        utxos.contains_key(&recipient_input),
        "expected UTxOs to include the transferred output"
    );

    // Keep the key around for follow-up assertions (e.g. submitting a
    // change-returning tx from the recipient). We don't use it yet, but
    // keeping the binding here makes the test future-proof.
    let _ = recipient_key;
}
