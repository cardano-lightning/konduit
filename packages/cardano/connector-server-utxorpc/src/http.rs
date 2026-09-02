use crate::error::ApiError;
use crate::ops::{OpsStore, Record};
use crate::providers::{History, Ledger, parse_mainnet_address};
use crate::tx::{decode_signed_tx, parse_lowercase_hex, parse_tx_id, parse_uuid};
use crate::wire::{
    BalanceResponse, CreateOperationRequest, HealthResponse, NetworkResponse,
    ProtocolParametersResponse, SubmitRequest, SubmitResponse, Utxo,
};
use actix_cors::Cors;
use actix_web::{
    App, HttpRequest, HttpResponse, HttpServer,
    middleware::Logger,
    web::{self, Data, Json, Path},
};
use serde::Serialize;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;
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

pub struct Limits {
    pub rate_per_minute: usize,
}

pub struct AppState<L, H> {
    pub ledger: L,
    pub history: H,
    pub ops: OpsStore,
    pub limits: Limits,
    pub hits: Mutex<HashMap<IpAddr, Vec<Instant>>>,
}

impl<L: Ledger, H: History> AppState<L, H> {
    fn rate_limit(&self, req: &HttpRequest) -> Result<(), ApiError> {
        let ip = req
            .peer_addr()
            .map(|addr| addr.ip())
            .unwrap_or(IpAddr::from([0, 0, 0, 0]));
        let mut hits = self.hits.lock().map_err(|_| ApiError::unexpected())?;
        let now = Instant::now();
        let entry = hits.entry(ip).or_default();
        entry.retain(|at| now.duration_since(*at) < Duration::from_secs(60));
        if entry.len() >= self.limits.rate_per_minute {
            return Err(ApiError::too_many());
        }
        entry.push(now);
        Ok(())
    }
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
    state.ledger.ready().await?;
    state.history.ping().await?;
    json_bounded(&HealthResponse { status: "ok" })
}

async fn network() -> Result<HttpResponse, ApiError> {
    json_bounded(&NetworkResponse { network: "mainnet" })
}

async fn protocol_parameters<L: Ledger, H: History>(
    state: Data<AppState<L, H>>,
) -> Result<HttpResponse, ApiError> {
    let (era, epoch, slot, payload) = state.ledger.protocol_parameters().await?;
    json_bounded(&ProtocolParametersResponse {
        era,
        epoch,
        slot,
        payload: payload.into(),
    })
}

async fn balance<L: Ledger, H: History>(
    state: Data<AppState<L, H>>,
    path: Path<String>,
) -> Result<HttpResponse, ApiError> {
    let address = parse_mainnet_address(&path)?;
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
}

async fn utxos_at<L: Ledger, H: History>(
    state: Data<AppState<L, H>>,
    path: Path<String>,
) -> Result<HttpResponse, ApiError> {
    let address = parse_mainnet_address(&path)?;
    let utxos: Vec<Utxo> = state.ledger.utxos_at(&address).await?;
    json_bounded(&utxos)
}

async fn transactions<L: Ledger, H: History>(
    state: Data<AppState<L, H>>,
    path: Path<String>,
) -> Result<HttpResponse, ApiError> {
    let address = parse_mainnet_address(&path)?;
    let (height, _) = state.ledger.tip().await?;
    let txs = state
        .history
        .address_history(&address.to_string(), height)
        .await?;
    json_bounded(&txs)
}

async fn transaction<L: Ledger, H: History>(
    state: Data<AppState<L, H>>,
    path: Path<String>,
) -> Result<HttpResponse, ApiError> {
    let id = path.into_inner();
    parse_tx_id(&id).map_err(|_| ApiError::bad_request())?;
    let (height, _) = state.ledger.tip().await?;
    let tx = state.history.transaction(&id, height).await?;
    json_bounded(&tx)
}

async fn submit<L: Ledger, H: History>(
    req: HttpRequest,
    state: Data<AppState<L, H>>,
    body: Json<SubmitRequest>,
) -> Result<HttpResponse, ApiError> {
    state.rate_limit(&req)?;
    let _guard = state.ops.admit_write()?;
    let bytes = parse_lowercase_hex(&body.transaction).map_err(|_| ApiError::bad_request())?;
    let signed = decode_signed_tx(&bytes).map_err(|_| ApiError::bad_request())?;
    let max = state.ledger.max_tx_size().await?;
    if signed.bytes.len() as u64 > max {
        return Err(ApiError::bad_request());
    }
    let uuid = format!(
        "00000000-0000-0000-0000-{}",
        hex::encode(&signed.hash[8..16])
    );
    // ponytail: legacy submit keys off txid via synthetic UUID; dedicated ops API uses client UUIDs
    let op_id = parse_uuid(&uuid).map_err(|_| ApiError::unexpected())?;
    let record = state
        .ops
        .persist_new(&op_id, &signed.hash, &signed, &uuid)?;
    submit_record(&state, &op_id, record).await?;
    json_bounded(&SubmitResponse {
        transaction_id: hex::encode(signed.hash),
    })
}

async fn create_operation<L: Ledger, H: History>(
    req: HttpRequest,
    state: Data<AppState<L, H>>,
    body: Json<CreateOperationRequest>,
) -> Result<HttpResponse, ApiError> {
    state.rate_limit(&req)?;
    let _guard = state.ops.admit_write()?;
    let op_id = parse_uuid(&body.operation_id).map_err(|_| ApiError::bad_request())?;
    let expected =
        parse_tx_id(&body.expected_transaction_id).map_err(|_| ApiError::bad_request())?;
    let bytes = parse_lowercase_hex(&body.transaction).map_err(|_| ApiError::bad_request())?;
    let signed = decode_signed_tx(&bytes).map_err(|_| ApiError::bad_request())?;
    if signed.hash != expected {
        return Err(ApiError::bad_request());
    }
    let max = state.ledger.max_tx_size().await?;
    if signed.bytes.len() as u64 > max {
        return Err(ApiError::bad_request());
    }
    let record = state
        .ops
        .persist_new(&op_id, &expected, &signed, &body.operation_id)?;
    let (depth, record) = submit_record(&state, &op_id, record).await?;
    json_bounded(&OpsStore::response(&record, depth))
}

async fn get_operation<L: Ledger, H: History>(
    state: Data<AppState<L, H>>,
    path: Path<String>,
) -> Result<HttpResponse, ApiError> {
    let op_id = parse_uuid(&path).map_err(|_| ApiError::bad_request())?;
    let mut record = state.ops.get(&op_id)?.ok_or_else(ApiError::not_found)?;
    let (height, slot) = state.ledger.tip().await?;
    let depth = state
        .ops
        .reconcile_one(&state.ledger, &op_id, &mut record, height, slot, false)
        .await?;
    json_bounded(&OpsStore::response(&record, depth))
}

async fn submit_record<L: Ledger, H: History>(
    state: &AppState<L, H>,
    op_id: &[u8; 16],
    mut record: Record,
) -> Result<(u64, Record), ApiError> {
    let (height, slot) = state.ledger.tip().await?;
    let depth = state
        .ops
        .reconcile_one(&state.ledger, op_id, &mut record, height, slot, true)
        .await?;
    Ok((depth, record))
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
    App::new()
        .app_data(state)
        .wrap(Logger::default())
        .wrap(
            Cors::default()
                .allow_any_origin()
                .allow_any_method()
                .allow_any_header(),
        )
        .configure(config::<L, H>)
}

pub async fn serve<L, H>(bind: &str, state: Data<AppState<L, H>>) -> std::io::Result<()>
where
    L: Ledger + 'static,
    H: History + 'static,
{
    HttpServer::new(move || app(state.clone()))
        .bind(bind)?
        .run()
        .await
}
