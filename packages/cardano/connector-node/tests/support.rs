//! Shared helpers for integration tests against a local Cardano testnet.
//!
//! Every public helper returns a `Result` whose `Err` is the standard
//! "missing env var" diagnostic; tests that call them gate on those errors
//! and `return` early so they are skipped (not failed) on machines without
//! the testnet plumbing.

use anyhow::{Context, anyhow};
use cardano_connector::CardanoConnector;
use cardano_connector_node::{Config, Node};
use cardano_sdk::cbor;
use cardano_sdk::{
    Address, Credential, Hash, Input, NetworkId, Output, ProtocolParameters, SigningKey, Value,
    address::kind,
};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::{Duration, Instant};

pub const ENV_SOCKET: &str = "CARDANO_NODE_SOCKET_PATH";
pub const ENV_NETWORK_ID: &str = "CARDANO_NODE_NETWORK_ID";
pub const ENV_FAUCET_ADDR: &str = "FAUCET_ADDR_FILE";
pub const ENV_FAUCET_SKEY_FILE: &str = "FAUCET_SKEY_FILE";
pub const ENV_FAUCET_WALLET_FILE: &str = "FAUCET_WALLET_FILE";

pub const DEFAULT_POLL_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Clone, Debug)]
pub struct TestEnv {
    pub socket: PathBuf,
    pub network_magic: u64,
    pub faucet_address: Address<kind::Shelley>,
    pub faucet_signing_key: SigningKey,
}

impl TestEnv {
    /// Read testnet plumbing from the environment. Returns `Err` if any of the
    /// expected variables is missing.
    pub fn from_env() -> anyhow::Result<Self> {
        let socket = std::env::var(ENV_SOCKET)
            .map(PathBuf::from)
            .with_context(|| format!("missing env var {ENV_SOCKET}"))?;
        let network_magic = std::env::var(ENV_NETWORK_ID)
            .map(|value| {
                value
                    .parse::<u64>()
                    .with_context(|| format!("invalid {ENV_NETWORK_ID}={value:?}"))
            })
            .context(format!("missing env var {ENV_NETWORK_ID}"))??;

        let faucet_address_file = std::env::var(ENV_FAUCET_ADDR)
            .with_context(|| format!("missing env var {ENV_FAUCET_ADDR}"))?;
        let faucet_address = std::fs::read_to_string(&faucet_address_file)
            .with_context(|| format!("failed to read faucet address from {faucet_address_file}"))?
            .trim()
            .parse::<Address<kind::Shelley>>()
            .with_context(|| {
                format!("failed to parse faucet address from {faucet_address_file}")
            })?;

        let faucet_signing_key = faucet_signing_key_from_env()
            .with_context(|| "failed to resolve faucet signing key")?;

        Ok(Self {
            socket,
            network_magic,
            faucet_address,
            faucet_signing_key,
        })
    }
}

#[derive(Deserialize)]
struct CardanoCliSkeyFile {
    #[serde(rename = "description")]
    _description: String,
    #[serde(rename = "cborHex")]
    cbor_hex: String,
    #[serde(rename = "type")]
    _type: String,
}

/// Try to load the faucet signing key from any of the well-known env vars.
fn faucet_signing_key_from_env() -> anyhow::Result<SigningKey> {
    if let Ok(path) = std::env::var(ENV_FAUCET_SKEY_FILE) {
        let skey_file = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read faucet signing key from {path}"))?;
        let skey_json: CardanoCliSkeyFile = serde_json::from_str(&skey_file)
            .with_context(|| format!("failed to parse faucet signing key from {path}"))?;
        let cbor_bytes = hex::decode(skey_json.cbor_hex.trim())
            .with_context(|| format!("failed to hex-decode faucet signing key from {path}"))?;
        let skey_bytes: Vec<u8> = cbor::decode(&cbor_bytes)
            .with_context(|| format!("failed to parse CBOR faucet signing key from {path}"))?;
        return SigningKey::try_from(skey_bytes)
            .with_context(|| format!("failed to convert faucet signing key from {path}"));
    }

    // Raw 64 chars hex
    if let Ok(path) = std::env::var(ENV_FAUCET_WALLET_FILE) {
        let skey_hex = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read faucet signing key from {path}"))?;
        let skey = SigningKey::from_str(skey_hex.trim())
            .with_context(|| format!("failed to parse faucet signing key from {path}"))?;
        return Ok(skey);
    }

    Err(anyhow!(
        "missing env var {ENV_FAUCET_SKEY_FILE} or {ENV_FAUCET_WALLET_FILE} for faucet signing key"
    ))
}

/// Build a [`Node`] connector from the env-loaded testnet. Performs a health
/// probe so the caller learns about handshake / version mismatches early.
pub async fn live_node() -> anyhow::Result<(TestEnv, Node)> {
    let env = TestEnv::from_env()?;
    let config = Config::new(env.socket.clone(), env.network_magic);
    let node = Node::connect(config).await?;
    node.health()
        .await
        .with_context(|| format!("local node at {} is not healthy", env.socket.display()))?;
    Ok((env, node))
}

pub fn fresh_keypair() -> (SigningKey, Address<kind::Shelley>) {
    let key = SigningKey::new();
    let address = key.to_verification_key().to_address(NetworkId::TESTNET);
    (key, address)
}

/// Poll `predicate` until it returns `true` or the deadline elapses.
pub async fn wait_until<F>(
    node: &Node,
    payment: &Credential,
    delegation: Option<&Credential>,
    predicate: F,
    deadline: Duration,
) -> anyhow::Result<BTreeMap<Input, Output>>
where
    F: Fn(&BTreeMap<Input, Output>) -> bool,
{
    let start = Instant::now();
    loop {
        let utxos = node.utxos_at(payment, delegation).await?;
        if predicate(&utxos) {
            return Ok(utxos);
        }
        if start.elapsed() >= deadline {
            return Err(anyhow!(
                "wait_until timed out after {:?} for {payment:?}; saw {} utxos",
                deadline,
                utxos.len()
            ));
        }
        tokio::time::sleep(DEFAULT_POLL_INTERVAL).await;
    }
}

/// Wait for a specific UTxO (matching `expected_input`) to appear in the set
/// of UTxOs owned by `payment`/`delegation`.
pub async fn wait_for_specific_utxo(
    node: &Node,
    payment: &Credential,
    delegation: Option<&Credential>,
    expected_input: &Input,
    deadline: Duration,
) -> anyhow::Result<BTreeMap<Input, Output>> {
    wait_until(
        node,
        payment,
        delegation,
        |utxos| utxos.contains_key(expected_input),
        deadline,
    )
    .await
}

/// Build, sign and submit an ada-only transfer from `sender_key` to
/// `recipient`. Returns the submitted transaction id so callers can correlate
/// the on-chain state with what was sent.
pub async fn transfer_ada(
    node: &Node,
    sender_key: &SigningKey,
    recipient: &Address<kind::Any>,
    lovelace: u64,
    params: &ProtocolParameters,
) -> anyhow::Result<Hash<32>> {
    let sender_address = sender_key
        .to_verification_key()
        .to_address(NetworkId::TESTNET);
    let sender_utxos = node
        .utxos_at(
            &sender_address.payment(),
            sender_address.delegation().as_ref(),
        )
        .await?;
    let change_address: Address<kind::Any> = sender_address.into();
    let mut tx = konduit_tx::admin::send(
        params,
        &sender_utxos,
        vec![(recipient.clone(), Value::new(lovelace))],
        change_address,
    )?;
    let tx_id = tx.id();
    tx.sign(sender_key);
    node.submit(&tx).await?;
    Ok(tx_id)
}

/// Wait until no UTxO matching `payment`/`delegation` exists. Useful for
/// asserting a UTxO has been spent (e.g. the channel being closed).
pub async fn wait_until_empty(
    node: &Node,
    payment: &Credential,
    delegation: Option<&Credential>,
    deadline: Duration,
) -> anyhow::Result<()> {
    let start = Instant::now();
    loop {
        let utxos = node.utxos_at(payment, delegation).await?;
        if utxos.is_empty() {
            return Ok(());
        }
        if start.elapsed() >= deadline {
            return Err(anyhow!(
                "wait_until_empty timed out after {:?} for {payment:?}; saw {} utxos",
                deadline,
                utxos.len()
            ));
        }
        tokio::time::sleep(DEFAULT_POLL_INTERVAL).await;
    }
}

/// Deploy the konduit validator referenced script to `host_address`. The
/// admin pays for it from its own UTxOs. Returns the tx id so callers can
/// correlate with on-chain state.
pub async fn deploy_validator(
    node: &Node,
    admin_key: &SigningKey,
    host_address: &Address<kind::Any>,
    params: &ProtocolParameters,
) -> anyhow::Result<Hash<32>> {
    let admin_address = admin_key
        .to_verification_key()
        .to_address(NetworkId::TESTNET);
    let admin_utxos = node
        .utxos_at(
            &admin_address.payment(),
            admin_address.delegation().as_ref(),
        )
        .await?;
    let change_address: Address<kind::Any> = admin_address.into();
    let mut tx = konduit_tx::admin::deploy(
        params,
        &admin_utxos,
        konduit_tx::KONDUIT_VALIDATOR.script.clone(),
        host_address.clone(),
        change_address,
    )?;
    let tx_id = tx.id();
    tx.sign(admin_key);
    node.submit(&tx).await?;
    Ok(tx_id)
}

/// Convenience: poll until at least one UTxO matches the predicate.
pub async fn wait_for_utxo_matching<F>(
    node: &Node,
    payment: &Credential,
    delegation: Option<&Credential>,
    predicate: F,
    deadline: Duration,
) -> anyhow::Result<BTreeMap<Input, Output>>
where
    F: Fn(&Output) -> bool + Clone,
{
    wait_until(
        node,
        payment,
        delegation,
        move |utxos| utxos.values().any(predicate.clone()),
        deadline,
    )
    .await
}
