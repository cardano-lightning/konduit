use crate::error::ApiError;
use crate::ops::{InternalState, OpsStore, Record};
use crate::providers::{History, Ledger, parse_mainnet_address};
use crate::tx::{decode_signed_tx, parse_lowercase_hex, parse_tx_id, parse_uuid};
use crate::wire::{
    BalanceResponse, HealthResponse, NetworkResponse, ProtocolParametersResponse, SubmitResponse,
    TransactionSummary, Utxo,
};
use actix_cors::Cors;
use actix_web::{
    App, HttpRequest, HttpResponse, HttpServer,
    error::JsonPayloadError,
    middleware::Logger,
    web::{self, Data, Json, JsonConfig, Path},
};
use futures_util::StreamExt;
use serde::Serialize;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub const OPENAPI_YAML: &str = include_str!("../openapi.yaml");
const MAX_JSON: usize = 1024 * 1024;
const DOCS_HTML: &str = r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <title>Cardano Connector Server API</title>
  </head>
  <body>
    <redoc spec-url="/openapi.yaml"></redoc>
    <script src="https://cdn.redoc.ly/redoc/latest/bundles/redoc.standalone.js"></script>
  </body>
</html>
"#;

pub(crate) fn legacy_uuid(hash: &[u8; 32]) -> String {
    format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{}",
        u32::from_be_bytes(hash[..4].try_into().unwrap()),
        u16::from_be_bytes(hash[4..6].try_into().unwrap()),
        u16::from_be_bytes(hash[6..8].try_into().unwrap()),
        u16::from_be_bytes(hash[8..10].try_into().unwrap()),
        hex::encode(&hash[10..16]),
    )
}

pub struct Limits {
    pub rate_per_minute: usize,
}

pub struct AppState<L, H> {
    pub ledger: Arc<L>,
    pub history: Arc<H>,
    pub ops: Arc<OpsStore>,
    pub limits: Limits,
    pub hits: Mutex<HashMap<IpAddr, Vec<Instant>>>,
}

impl<L: Ledger, H: History> AppState<L, H> {
    fn rate_limit(&self, req: &HttpRequest) -> Result<(), ApiError> {
        let forwarded = if trust_proxy() {
            req.headers()
                .get("x-forwarded-for")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.split(',').next())
                .and_then(|value| value.trim().parse().ok())
        } else {
            None
        };
        let ip = forwarded
            .or_else(|| req.peer_addr().map(|addr| addr.ip()))
            .unwrap_or(IpAddr::from([0, 0, 0, 0]));
        let mut hits = self.hits.lock().map_err(|_| ApiError::unexpected())?;
        let now = Instant::now();
        hits.retain(|_, entry| {
            entry.retain(|at| now.duration_since(*at) < Duration::from_secs(60));
            !entry.is_empty()
        });
        if !hits.contains_key(&ip) && hits.len() >= 10_000 {
            return Err(ApiError::too_many());
        }
        let entry = hits.entry(ip).or_default();
        if entry.len() >= self.limits.rate_per_minute {
            return Err(ApiError::too_many());
        }
        entry.push(now);
        Ok(())
    }
}

fn trust_proxy() -> bool {
    matches!(
        std::env::var("CONNECTOR_TRUST_PROXY").as_deref(),
        Ok("1" | "true" | "TRUE" | "yes" | "YES")
    )
}

fn json_bounded<T: Serialize>(value: &T) -> Result<HttpResponse, ApiError> {
    let body = serde_json::to_vec(value).map_err(|_| ApiError::unexpected())?;
    if body.len() > MAX_JSON {
        return Err(ApiError::payload());
    }
    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .body(body))
}

const REQUEST_BUDGET: Duration = Duration::from_secs(19);

async fn bounded<F, T>(fut: F) -> Result<T, ApiError>
where
    F: Future<Output = Result<T, ApiError>>,
{
    tokio::time::timeout(REQUEST_BUDGET, fut)
        .await
        .map_err(|_| ApiError::unavailable())?
}

async fn docs() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(DOCS_HTML)
}

async fn openapi() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/yaml; charset=utf-8")
        .body(OPENAPI_YAML)
}

async fn health<L: Ledger, H: History>(
    state: Data<AppState<L, H>>,
) -> Result<HttpResponse, ApiError> {
    bounded(async {
        state.ledger.ready().await?;
        let (height, _) = state.ledger.tip().await?;
        state.history.ready(height).await?;
        state.ops.ready().await?;
        json_bounded(&HealthResponse { status: "ok" })
    })
    .await
}

async fn network() -> Result<HttpResponse, ApiError> {
    json_bounded(&NetworkResponse { network: "mainnet" })
}

async fn protocol_parameters<L: Ledger, H: History>(
    state: Data<AppState<L, H>>,
) -> Result<HttpResponse, ApiError> {
    bounded(async {
        let (era, epoch, slot, payload) = state.ledger.protocol_parameters().await?;
        json_bounded(&ProtocolParametersResponse {
            era,
            epoch,
            slot,
            payload: payload.into(),
        })
    })
    .await
}

async fn balance<L: Ledger, H: History>(
    state: Data<AppState<L, H>>,
    path: Path<String>,
) -> Result<HttpResponse, ApiError> {
    let address = parse_mainnet_address(&path)?;
    bounded(async {
        let utxos = state.ledger.utxos_at(&address).await?;
        let mut total: u128 = 0;
        for utxo in utxos {
            for asset in utxo.value {
                if asset.unit == "lovelace" {
                    let qty: u128 = asset.quantity.parse().map_err(|_| ApiError::unexpected())?;
                    total = total.checked_add(qty).ok_or_else(ApiError::unexpected)?;
                }
            }
        }
        json_bounded(&BalanceResponse {
            lovelace: total.to_string(),
        })
    })
    .await
}

async fn utxos_at<L: Ledger, H: History>(
    state: Data<AppState<L, H>>,
    path: Path<String>,
) -> Result<HttpResponse, ApiError> {
    let address = parse_mainnet_address(&path)?;
    bounded(async {
        let utxos: Vec<Utxo> = state.ledger.utxos_at(&address).await?;
        json_bounded(&utxos)
    })
    .await
}

async fn transactions<L: Ledger, H: History>(
    state: Data<AppState<L, H>>,
    path: Path<String>,
) -> Result<HttpResponse, ApiError> {
    let address = parse_mainnet_address(&path)?;
    bounded(async {
        let (height, _) = state.ledger.tip().await?;
        let txs = state.history.address_history(&address.to_string()).await?;
        let mut checks = futures_util::stream::iter(
            txs.into_iter()
                .map(|tx| confirm_canonical(state.ledger.as_ref(), tx, height)),
        )
        .buffered(8);
        let mut out = Vec::new();
        while let Some(result) = checks.next().await {
            if let Some(tx) = result? {
                out.push(tx);
            }
        }
        json_bounded(&out)
    })
    .await
}

async fn transaction<L: Ledger, H: History>(
    state: Data<AppState<L, H>>,
    path: Path<String>,
) -> Result<HttpResponse, ApiError> {
    let id = path.into_inner();
    parse_tx_id(&id).map_err(|_| ApiError::bad_request())?;
    bounded(async {
        let (height, _) = state.ledger.tip().await?;
        let tx = state.history.transaction(&id).await?;
        let tx = match tx {
            Some(tx) if tx.id == id => confirm_canonical(state.ledger.as_ref(), tx, height).await?,
            Some(_) => return Err(ApiError::unavailable()),
            None => None,
        };
        json_bounded(&tx)
    })
    .await
}

async fn submit<L: Ledger, H: History>(
    req: HttpRequest,
    state: Data<AppState<L, H>>,
    body: Json<crate::wire::SubmitRequest>,
) -> Result<HttpResponse, ApiError> {
    state.rate_limit(&req)?;
    let _admission = state.ops.admit_write()?;
    bounded(async {
        let bytes = parse_lowercase_hex(&body.transaction).map_err(|_| ApiError::bad_request())?;
        let signed = decode_signed_tx(&bytes).map_err(|_| ApiError::bad_request())?;
        let max = state.ledger.max_tx_size().await?;
        if signed.bytes.len() as u64 > max {
            return Err(ApiError::bad_request());
        }
        let uuid = legacy_uuid(&signed.hash);
        let op_id = parse_uuid(&uuid).map_err(|_| ApiError::unexpected())?;
        let key = OpsStore::legacy_key(&op_id);
        let record = state
            .ops
            .persist_new(key, signed.hash, signed.clone(), uuid)
            .await?;
        let record = submit_record(&state, key, record).await?;
        if record.state == InternalState::Rejected {
            return Err(ApiError::bad_request());
        }
        json_bounded(&SubmitResponse {
            transaction_id: hex::encode(signed.hash),
        })
    })
    .await
}

async fn create_operation<L: Ledger, H: History>(
    req: HttpRequest,
    state: Data<AppState<L, H>>,
    body: Json<crate::wire::CreateOperationRequest>,
) -> Result<HttpResponse, ApiError> {
    state.rate_limit(&req)?;
    let _admission = state.ops.admit_write()?;
    let body = body.into_inner();
    let op_id = parse_uuid(&body.operation_id).map_err(|_| ApiError::bad_request())?;
    let expected =
        parse_tx_id(&body.expected_transaction_id).map_err(|_| ApiError::bad_request())?;
    let bytes = parse_lowercase_hex(&body.transaction).map_err(|_| ApiError::bad_request())?;
    let signed = decode_signed_tx(&bytes).map_err(|_| ApiError::bad_request())?;
    if signed.hash != expected {
        return Err(ApiError::bad_request());
    }
    let key = OpsStore::client_key(&op_id);
    if state.ops.get(key).await?.is_some() {
        let record = state
            .ops
            .persist_new(key, expected, signed, body.operation_id)
            .await?;
        return json_bounded(&OpsStore::response(&record));
    }

    bounded(async {
        let (_, tip_slot) = state.ledger.tip().await?;
        if signed.ttl.is_none_or(|ttl| ttl <= tip_slot) {
            return Err(ApiError::bad_request());
        }
        let max = state.ledger.max_tx_size().await?;
        if signed.bytes.len() as u64 > max {
            return Err(ApiError::bad_request());
        }
        let record = state
            .ops
            .persist_new(key, expected, signed, body.operation_id)
            .await?;
        let record = submit_record(&state, key, record).await?;
        json_bounded(&OpsStore::response(&record))
    })
    .await
}

async fn get_operation<L: Ledger, H: History>(
    state: Data<AppState<L, H>>,
    path: Path<String>,
) -> Result<HttpResponse, ApiError> {
    let op_id = parse_uuid(&path).map_err(|_| ApiError::bad_request())?;
    let key = OpsStore::client_key(&op_id);
    let mut record = state.ops.get(key).await?.ok_or_else(ApiError::not_found)?;
    if matches!(
        record.state,
        InternalState::Settled | InternalState::Rejected
    ) {
        return json_bounded(&OpsStore::response(&record));
    }
    bounded(async {
        let (height, slot) = state.ledger.tip().await?;
        state
            .ops
            .reconcile_one(state.ledger.as_ref(), key, &mut record, height, slot, false)
            .await?;
        json_bounded(&OpsStore::response(&record))
    })
    .await
}

async fn confirm_canonical<L: Ledger>(
    ledger: &L,
    mut tx: TransactionSummary,
    tip_height: u64,
) -> Result<Option<TransactionSummary>, ApiError> {
    let id = parse_tx_id(&tx.id).map_err(|_| ApiError::unavailable())?;
    let Some(height) = ledger.read_tx(&id).await? else {
        return Ok(None);
    };
    if height != tx.block_height {
        return Err(ApiError::unavailable());
    }
    if height > tip_height {
        log::warn!("Dolos transaction height is ahead of its current tip");
    }
    tx.depth = tip_height.saturating_sub(height);
    Ok(Some(tx))
}
async fn submit_record<L: Ledger, H: History>(
    state: &AppState<L, H>,
    key: crate::ops::OperationKey,
    mut record: Record,
) -> Result<Record, ApiError> {
    let (height, slot) = state.ledger.tip().await?;
    let submit = record.state == InternalState::Prepared;
    state
        .ops
        .reconcile_one(
            state.ledger.as_ref(),
            key,
            &mut record,
            height,
            slot,
            submit,
        )
        .await?;
    Ok(record)
}

pub fn config<L, H>(cfg: &mut web::ServiceConfig)
where
    L: Ledger + 'static,
    H: History + 'static,
{
    cfg.route("/", web::get().to(docs))
        .route("/openapi.yaml", web::get().to(openapi))
        .route("/health", web::get().to(health::<L, H>))
        .route("/network", web::get().to(network))
        .route(
            "/protocol-parameters",
            web::get().to(protocol_parameters::<L, H>),
        )
        .route("/balance/{address}", web::get().to(balance::<L, H>))
        .route("/utxos_at/{address}", web::get().to(utxos_at::<L, H>))
        .route(
            "/transactions/{address}",
            web::get().to(transactions::<L, H>),
        )
        .route("/transaction/{id}", web::get().to(transaction::<L, H>))
        .route("/submit", web::post().to(submit::<L, H>))
        .route("/operations", web::post().to(create_operation::<L, H>))
        .route(
            "/operations/{operation_id}",
            web::get().to(get_operation::<L, H>),
        );
}

pub fn app<L, H>(
    state: Data<AppState<L, H>>,
) -> App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse<impl actix_web::body::MessageBody>,
        Error = actix_web::Error,
        InitError = (),
    >,
>
where
    L: Ledger + 'static,
    H: History + 'static,
{
    let cors = if let Ok(origin) = std::env::var("CONNECTOR_CORS_ORIGIN") {
        Cors::default()
            .allowed_origin(&origin)
            .allow_any_method()
            .allow_any_header()
            .block_on_origin_mismatch(true)
    } else {
        Cors::default().block_on_origin_mismatch(true)
    };
    App::new()
        .app_data(state)
        .app_data(
            JsonConfig::default()
                .limit(MAX_JSON)
                .error_handler(|err, _| {
                    let overflow = matches!(
                        err,
                        JsonPayloadError::Overflow { .. }
                            | JsonPayloadError::OverflowKnownLength { .. }
                    );
                    if overflow {
                        ApiError::payload().into()
                    } else {
                        ApiError::bad_request().into()
                    }
                }),
        )
        .wrap(Logger::new("%s %b %D"))
        .wrap(cors)
        .configure(config::<L, H>)
}

pub async fn serve<L, H>(bind: &str, state: Data<AppState<L, H>>) -> std::io::Result<()>
where
    L: Ledger + 'static,
    H: History + 'static,
{
    HttpServer::new(move || app(state.clone()))
        .client_request_timeout(REQUEST_BUDGET)
        .bind(bind)?
        .run()
        .await
}
