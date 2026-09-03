use crate::http::{AppState, Limits, app, legacy_uuid};
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
use std::sync::{Arc, Mutex};

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
        ledger: Arc::new(ledger),
        history: Arc::new(history),
        ops: Arc::new(tmp_db()),
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
async fn write_routes_reject_non_json_before_parsing() {
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
    let response = test::call_service(
        &app,
        TestRequest::post()
            .uri("/operations")
            .insert_header(("content-type", "text/plain"))
            .set_payload("{}")
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
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

#[actix_web::test]
async fn operation_conflict_and_idempotency() {
    let store = tmp_db();
    let uuid = "550e8400-e29b-41d4-a716-446655440000";
    let op = OpsStore::client_key(&parse_uuid(uuid).unwrap());
    let txid = [1u8; 32];
    let signed = SignedTx {
        hash: txid,
        digest: [2u8; 32],
        ttl: Some(10),
        bytes: vec![1, 2, 3],
    };
    let first = store
        .persist_new(op, txid, signed.clone(), uuid.to_owned())
        .await
        .unwrap();
    let again = store
        .persist_new(op, txid, signed.clone(), uuid.to_owned())
        .await
        .unwrap();
    assert_eq!(first.digest, again.digest);
    let mut other = signed.clone();
    other.digest = [9u8; 32];
    assert!(
        store
            .persist_new(op, txid, other, uuid.to_owned())
            .await
            .is_err()
    );
    let other_op =
        OpsStore::client_key(&parse_uuid("550e8400-e29b-41d4-a716-446655440001").unwrap());
    assert!(
        store
            .persist_new(
                other_op,
                txid,
                signed,
                "550e8400-e29b-41d4-a716-446655440001".to_owned()
            )
            .await
            .is_err()
    );
}

#[actix_web::test]
async fn claim_submit_is_exclusive() {
    let store = tmp_db();
    let uuid = "550e8400-e29b-41d4-a716-446655440002";
    let key = OpsStore::client_key(&parse_uuid(uuid).unwrap());
    let txid = [3u8; 32];
    let signed = SignedTx {
        hash: txid,
        digest: [4u8; 32],
        ttl: Some(99),
        bytes: vec![9, 9, 9],
    };
    store
        .persist_new(key, txid, signed, uuid.to_owned())
        .await
        .unwrap();
    assert!(store.claim_submit(key).await.unwrap().is_some());
    assert!(store.claim_submit(key).await.unwrap().is_none());
}

#[actix_web::test]
async fn confirmed_keeps_cbor() {
    let store = tmp_db();
    let uuid = "550e8400-e29b-41d4-a716-446655440003";
    let key = OpsStore::client_key(&parse_uuid(uuid).unwrap());
    let txid = [5u8; 32];
    let signed = SignedTx {
        hash: txid,
        digest: [6u8; 32],
        ttl: Some(99),
        bytes: vec![9, 9, 9],
    };
    let mut record = store
        .persist_new(key, txid, signed, uuid.to_owned())
        .await
        .unwrap();
    let mut txs = HashMap::new();
    txs.insert(txid, TxPresence { height: 95 });
    let ledger = FakeLedger {
        txs,
        ..default_ledger()
    };
    store
        .reconcile_one(&ledger, key, &mut record, 100, 50_000, false)
        .await
        .unwrap();
    assert_eq!(record.state, InternalState::Confirmed);
    assert!(record.cbor.is_some());
}

#[actix_web::test]
async fn settled_is_not_reset() {
    let store = tmp_db();
    let uuid = "550e8400-e29b-41d4-a716-446655440004";
    let key = OpsStore::client_key(&parse_uuid(uuid).unwrap());
    let txid = [7u8; 32];
    let signed = SignedTx {
        hash: txid,
        digest: [8u8; 32],
        ttl: Some(99),
        bytes: vec![9, 9, 9],
    };
    let mut record = store
        .persist_new(key, txid, signed, uuid.to_owned())
        .await
        .unwrap();
    record.state = InternalState::Settled;
    record.cbor = None;
    store.put(key, &mut record).await.unwrap();
    store
        .reconcile_one(&default_ledger(), key, &mut record, 100, 50_000, true)
        .await
        .unwrap();
    assert_eq!(record.state, InternalState::Settled);
}

#[actix_web::test]
async fn stale_revision_cannot_roll_back_newer_state() {
    let store = tmp_db();
    let uuid = "550e8400-e29b-41d4-a716-446655440005";
    let key = OpsStore::client_key(&parse_uuid(uuid).unwrap());
    let txid = [10u8; 32];
    let signed = SignedTx {
        hash: txid,
        digest: [11u8; 32],
        ttl: Some(99),
        bytes: vec![9, 9, 9],
    };
    let mut stale = store
        .persist_new(key, txid, signed, uuid.to_owned())
        .await
        .unwrap();
    let mut current = stale.clone();
    current.state = InternalState::Accepted;
    store.put(key, &mut current).await.unwrap();
    stale.state = InternalState::Prepared;
    store.put(key, &mut stale).await.unwrap();
    assert_eq!(stale.state, InternalState::Accepted);
    assert_eq!(
        store.get(key).await.unwrap().unwrap().state,
        InternalState::Accepted
    );
}

#[actix_web::test]
async fn confirmed_can_regress_to_accepted() {
    let store = tmp_db();
    let uuid = "550e8400-e29b-41d4-a716-446655440006";
    let key = OpsStore::client_key(&parse_uuid(uuid).unwrap());
    let txid = [12u8; 32];
    let signed = SignedTx {
        hash: txid,
        digest: [13u8; 32],
        ttl: Some(99),
        bytes: vec![9, 9, 9],
    };
    let mut record = store
        .persist_new(key, txid, signed, uuid.to_owned())
        .await
        .unwrap();
    record.state = InternalState::Confirmed;
    store.put(key, &mut record).await.unwrap();
    record.state = InternalState::Accepted;
    store.put(key, &mut record).await.unwrap();
    assert_eq!(record.state, InternalState::Accepted);
}

#[actix_web::test]
async fn legacy_operation_without_ttl_expires() {
    let store = tmp_db();
    let uuid = "550e8400-e29b-41d4-a716-446655440006";
    let key = OpsStore::legacy_key(&parse_uuid(uuid).unwrap());
    let txid = [12u8; 32];
    let signed = SignedTx {
        hash: txid,
        digest: [13u8; 32],
        ttl: None,
        bytes: vec![9, 9, 9],
    };
    let mut record = store
        .persist_new(key, txid, signed, uuid.to_owned())
        .await
        .unwrap();
    record.created_at_epoch_secs = 0;
    store.put(key, &mut record).await.unwrap();
    store
        .reconcile_one(&default_ledger(), key, &mut record, 100, 50_000, false)
        .await
        .unwrap();
    assert_eq!(record.state, InternalState::Rejected);
    assert!(record.cbor.is_none());
}
#[test]
fn legacy_submit_uuid_is_canonical() {
    let hash: [u8; 32] = std::array::from_fn(|index| index as u8);
    let uuid = legacy_uuid(&hash);
    assert_eq!(uuid, "00010203-0405-0607-0809-0a0b0c0d0e0f");
    assert_eq!(parse_uuid(&uuid).unwrap(), hash[..16]);
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
