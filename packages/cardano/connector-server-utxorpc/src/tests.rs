use crate::http::{AppState, Limits, app};
use crate::ops::{InternalState, OpsStore, finality};
use crate::providers::{History, Ledger, SubmitResult, TxPresence};
use crate::tx::{SignedTx, parse_uuid};
use crate::wire::{TransactionSummary, Utxo};
use actix_web::{
    http::StatusCode,
    test::{self, TestRequest},
    web::Data,
};
use async_trait::async_trait;
use cardano_connector_utxorpc::BloxbeanPayload;
use cardano_sdk::{Address, address::kind};
use std::collections::HashMap;
use std::sync::Mutex;

const MAINNET_ADDR: &str = "addr1vy2q4s9vxk3q8l0xq0l0xq0l0xq0l0xq0l0xq0l0xq0l0xq0l0xq0l0";

struct FakeLedger {
    tip: (u64, u64),
    ready: bool,
    utxos: Vec<Utxo>,
    txs: HashMap<[u8; 32], TxPresence>,
    submits: Mutex<Vec<Vec<u8>>>,
    max_tx_size: u64,
    params: BloxbeanPayload,
}

struct FakeHistory {
    ping: bool,
    by_id: HashMap<String, TransactionSummary>,
    by_addr: HashMap<String, Vec<TransactionSummary>>,
    fail: bool,
}

#[async_trait]
impl Ledger for FakeLedger {
    async fn ready(&self) -> Result<(), crate::ApiError> {
        if self.ready {
            Ok(())
        } else {
            Err(crate::ApiError::unavailable())
        }
    }

    async fn tip(&self) -> Result<(u64, u64), crate::ApiError> {
        Ok(self.tip)
    }

    async fn protocol_parameters(
        &self,
    ) -> Result<(String, u64, u64, BloxbeanPayload), crate::ApiError> {
        Ok(("Conway".into(), 500, self.tip.1, self.params.clone()))
    }

    async fn utxos_at(&self, _: &Address<kind::Shelley>) -> Result<Vec<Utxo>, crate::ApiError> {
        Ok(self.utxos.clone())
    }

    async fn read_tx(&self, txid: &[u8; 32]) -> Result<Option<TxPresence>, crate::ApiError> {
        Ok(self.txs.get(txid).cloned())
    }

    async fn submit_cbor(&self, cbor: &[u8]) -> Result<SubmitResult, crate::ApiError> {
        self.submits.lock().expect("lock").push(cbor.to_vec());
        Ok(SubmitResult::Accepted({
            let mut hash = [0u8; 32];
            hash.copy_from_slice(&cbor[..32.min(cbor.len())]);
            hash
        }))
    }

    async fn max_tx_size(&self) -> Result<u64, crate::ApiError> {
        Ok(self.max_tx_size)
    }
}

#[async_trait]
impl History for FakeHistory {
    async fn ping(&self) -> Result<(), crate::ApiError> {
        if self.ping && !self.fail {
            Ok(())
        } else {
            Err(crate::ApiError::unavailable())
        }
    }

    async fn address_history(
        &self,
        address: &str,
        _: u64,
    ) -> Result<Vec<TransactionSummary>, crate::ApiError> {
        if self.fail {
            return Err(crate::ApiError::unavailable());
        }
        Ok(self.by_addr.get(address).cloned().unwrap_or_default())
    }

    async fn transaction(
        &self,
        txid: &str,
        _: u64,
    ) -> Result<Option<TransactionSummary>, crate::ApiError> {
        if self.fail {
            return Err(crate::ApiError::unavailable());
        }
        Ok(self.by_id.get(txid).cloned())
    }
}

fn payload() -> BloxbeanPayload {
    BloxbeanPayload {
        min_fee_a: 44,
        min_fee_b: 155381,
        max_tx_size: 16384,
        key_deposit: 2_000_000,
        pool_deposit: 500_000_000,
        min_pool_cost: 170_000_000,
        protocol_major_ver: 10,
        protocol_minor_ver: 0,
        coins_per_utxo_size: 4310,
        collateral_percent: 150,
        max_collateral_inputs: 3,
    }
}

fn tmp_db() -> OpsStore {
    let path = std::env::temp_dir().join(format!(
        "connector-ops-{}-{}.redb",
        std::process::id(),
        now_nonce()
    ));
    OpsStore::open(&path, 8, 10_000_000, 8).expect("db")
}

fn now_nonce() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(1);
    N.fetch_add(1, Ordering::Relaxed)
}

fn state(ledger: FakeLedger, history: FakeHistory) -> Data<AppState<FakeLedger, FakeHistory>> {
    Data::new(AppState {
        ledger,
        history,
        ops: tmp_db(),
        limits: Limits {
            rate_per_minute: 1_000,
        },
        hits: Mutex::new(Default::default()),
    })
}

fn default_ledger() -> FakeLedger {
    FakeLedger {
        tip: (100, 50_000),
        ready: true,
        utxos: vec![],
        txs: HashMap::new(),
        submits: Mutex::new(Vec::new()),
        max_tx_size: 16_384,
        params: payload(),
    }
}

#[actix_web::test]
async fn health_and_network() {
    let app = test::init_service(app(state(
        default_ledger(),
        FakeHistory {
            ping: true,
            by_id: HashMap::new(),
            by_addr: HashMap::new(),
            fail: false,
        },
    )))
    .await;
    let health = test::call_service(&app, TestRequest::get().uri("/health").to_request()).await;
    assert_eq!(health.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(health).await;
    assert_eq!(body, serde_json::json!({"status":"ok"}));

    let network = test::call_service(&app, TestRequest::get().uri("/network").to_request()).await;
    let body: serde_json::Value = test::read_body_json(network).await;
    assert_eq!(body, serde_json::json!({"network":"mainnet"}));
}

#[actix_web::test]
async fn health_fails_closed() {
    let mut ledger = default_ledger();
    ledger.ready = false;
    let app = test::init_service(app(state(
        ledger,
        FakeHistory {
            ping: true,
            by_id: HashMap::new(),
            by_addr: HashMap::new(),
            fail: false,
        },
    )))
    .await;
    let health = test::call_service(&app, TestRequest::get().uri("/health").to_request()).await;
    assert_eq!(health.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[actix_web::test]
async fn protocol_parameters_from_ledger() {
    let app = test::init_service(app(state(
        default_ledger(),
        FakeHistory {
            ping: true,
            by_id: HashMap::new(),
            by_addr: HashMap::new(),
            fail: false,
        },
    )))
    .await;
    let res = test::call_service(
        &app,
        TestRequest::get().uri("/protocol-parameters").to_request(),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["payload"]["min_fee_a"], 44);
    assert_eq!(body["payload"]["key_deposit"], "2000000");
}

#[actix_web::test]
async fn missing_transaction_is_null() {
    let app = test::init_service(app(state(
        default_ledger(),
        FakeHistory {
            ping: true,
            by_id: HashMap::new(),
            by_addr: HashMap::new(),
            fail: false,
        },
    )))
    .await;
    let res = test::call_service(
        &app,
        TestRequest::get()
            .uri("/transaction/0000000000000000000000000000000000000000000000000000000000000000")
            .to_request(),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert!(body.is_null());
}

#[actix_web::test]
async fn koios_failure_is_unavailable() {
    let app = test::init_service(app(state(
        default_ledger(),
        FakeHistory {
            ping: true,
            by_id: HashMap::new(),
            by_addr: HashMap::new(),
            fail: true,
        },
    )))
    .await;
    let res = test::call_service(
        &app,
        TestRequest::get()
            .uri("/transaction/0000000000000000000000000000000000000000000000000000000000000000")
            .to_request(),
    )
    .await;
    assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[actix_web::test]
async fn operations_reject_unknown_fields() {
    let app = test::init_service(app(state(
        default_ledger(),
        FakeHistory {
            ping: true,
            by_id: HashMap::new(),
            by_addr: HashMap::new(),
            fail: false,
        },
    )))
    .await;
    let res = test::call_service(
        &app,
        TestRequest::post()
            .uri("/operations")
            .set_json(serde_json::json!({
                "operation_id": "550e8400-e29b-41d4-a716-446655440000",
                "expected_transaction_id": "00".repeat(32),
                "transaction": "aa",
                "extra": true
            }))
            .to_request(),
    )
    .await;
    assert!(res.status().is_client_error());
}

#[actix_web::test]
async fn openapi_is_served() {
    let app = test::init_service(app(state(
        default_ledger(),
        FakeHistory {
            ping: true,
            by_id: HashMap::new(),
            by_addr: HashMap::new(),
            fail: false,
        },
    )))
    .await;
    let res = test::call_service(&app, TestRequest::get().uri("/openapi.yaml").to_request()).await;
    assert_eq!(res.status(), StatusCode::OK);
    let body = test::read_body(res).await;
    assert!(std::str::from_utf8(&body).unwrap().contains("mainnet"));
}

#[test]
fn operation_conflict_and_idempotency() {
    let store = tmp_db();
    let uuid = "550e8400-e29b-41d4-a716-446655440000";
    let op = parse_uuid(uuid).unwrap();
    let txid = [1u8; 32];
    let signed = SignedTx {
        hash: txid,
        digest: [2u8; 32],
        ttl: Some(10),
        bytes: vec![1, 2, 3],
    };
    let first = store.persist_new(&op, &txid, &signed, uuid).unwrap();
    let again = store.persist_new(&op, &txid, &signed, uuid).unwrap();
    assert_eq!(first.digest, again.digest);
    let mut other = signed.clone();
    other.digest = [9u8; 32];
    assert!(store.persist_new(&op, &txid, &other, uuid).is_err());
    let other_op = parse_uuid("550e8400-e29b-41d4-a716-446655440001").unwrap();
    assert!(
        store
            .persist_new(
                &other_op,
                &txid,
                &signed,
                "550e8400-e29b-41d4-a716-446655440001"
            )
            .is_err()
    );
}

#[test]
fn crash_before_submit_stays_prepared() {
    let store = tmp_db();
    let uuid = "550e8400-e29b-41d4-a716-446655440002";
    let op = parse_uuid(uuid).unwrap();
    let txid = [3u8; 32];
    let signed = SignedTx {
        hash: txid,
        digest: [4u8; 32],
        ttl: Some(99),
        bytes: vec![9, 9, 9],
    };
    let record = store.persist_new(&op, &txid, &signed, uuid).unwrap();
    assert_eq!(record.state, InternalState::Prepared);
    let loaded = store.get(&op).unwrap().unwrap();
    assert_eq!(loaded.state, InternalState::Prepared);
    assert!(loaded.cbor.is_some());
}

#[test]
fn finality_edges() {
    assert_eq!(finality(4), InternalState::Accepted);
    assert_eq!(finality(5), InternalState::Confirmed);
    assert_eq!(finality(2159), InternalState::Confirmed);
    assert_eq!(finality(2160), InternalState::Settled);
}

#[test]
fn ensure_network_helper_rejects_non_mainnet() {
    let err = cardano_connector_utxorpc::ensure_network_matches(
        cardano_sdk::Network::Mainnet,
        cardano_sdk::Network::Preprod,
        "http://127.0.0.1:1337",
    )
    .unwrap_err();
    assert!(err.to_string().contains("does not match"));
}
