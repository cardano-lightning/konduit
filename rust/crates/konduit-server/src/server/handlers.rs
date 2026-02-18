use crate::{
    db, info,
    models::{PayBody, QuoteBody, SquashResponse},
    server::{self, cbor::decode_from_cbor},
};
use actix_web::{HttpMessage, HttpRequest, HttpResponse, ResponseError, http::StatusCode, web};
use konduit_data::{Keytag, Locked, Secret, Squash};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

type Data = web::Data<server::Data>;

const FEE_PLACEHOLDER: u64 = 1000;

#[derive(Debug, thiserror::Error)]
pub enum HandlerError {
    #[error("Internal Network Error")]
    Network(#[from] reqwest::Error),

    #[error("LND returned: {0}")]
    LndApi(String),

    #[error("DB returned: {0}")]
    Db(#[from] db::Error),

    #[error("Other")]
    Other,
}

impl ResponseError for HandlerError {
    fn status_code(&self) -> StatusCode {
        match self {
            HandlerError::Network(_) => StatusCode::INTERNAL_SERVER_ERROR,
            HandlerError::LndApi(_) => StatusCode::BAD_GATEWAY,
            HandlerError::Db(_) => StatusCode::INTERNAL_SERVER_ERROR,
            HandlerError::Other => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .json(serde_json::json!({ "error": self.to_string() }))
    }
}

// TODO :: MOVE TO CONFIG
/// This is ~ the same as the default on bitcoin: default (apparently) is 40 blocks
const ADAPTOR_TIME_DELTA: std::time::Duration = Duration::from_secs(40 * 10 * 60);
/// "Grace" is extra time between the "quoted" rel time and the time that might be allowed for in a
/// "pay"
const ADAPTOR_TIME_GRACE: std::time::Duration = Duration::from_secs(10 * 60);

pub async fn info(data: Data) -> info::Info {
    (*data.info()).clone()
}

pub async fn fx(data: Data) -> HttpResponse {
    let fx = data.fx().read().await.clone();
    HttpResponse::Ok().json(fx)
}

pub async fn show(data: Data) -> Result<HttpResponse, HandlerError> {
    log::info!("SHOW");
    let results = data.db().get_all().await?;
    Ok(HttpResponse::Ok().json(results))
}

pub async fn squash(
    req: HttpRequest,
    data: Data,
    body: web::Bytes,
) -> Result<HttpResponse, HandlerError> {
    let Some(keytag) = req.extensions().get::<Keytag>().cloned() else {
        return Ok(HttpResponse::InternalServerError().body("Error: Middleware data not found."));
    };

    let squash: Squash = match decode_from_cbor(body.as_ref()) {
        Ok(squash) => squash,
        Err(err) => {
            return Ok(HttpResponse::BadRequest().body(format!("cannot decode squash: {err}")));
        }
    };
    let (key, tag) = keytag.split();
    if !squash.verify(&key, &tag) {
        return Ok(HttpResponse::BadRequest().body("Invalid squash"));
    }
    let channel = data.db().update_squash(&keytag, squash).await?;
    let Some(receipt) = channel.receipt() else {
        return Ok(HttpResponse::InternalServerError().body("Impossible result"));
    };
    let response_body = match receipt.squash_proposal() {
        Ok(Some(propose)) => SquashResponse::Incomplete(propose),
        Ok(None) => SquashResponse::Complete,
        Err(_) => {
            return Ok(HttpResponse::InternalServerError().body("Failed to resolve squash"));
        }
    };
    Ok(HttpResponse::Ok().json(response_body))
}

pub async fn quote(
    req: HttpRequest,
    data: Data,
    body: web::Json<QuoteBody>,
) -> Result<HttpResponse, HandlerError> {
    let Some(keytag) = req.extensions().get::<Keytag>().cloned() else {
        return Ok(HttpResponse::InternalServerError().body("Error: Middleware data not found."));
    };
    let fx = data.fx().read().await.clone();
    let Some(channel) = data.db().get_channel(&keytag).await? else {
        return Ok(HttpResponse::BadRequest().body("No channel found"));
    };
    let Ok(_capacity) = channel.capacity() else {
        return Ok(HttpResponse::BadRequest().body("No capacity"));
    };
    let Ok(index) = channel.next_index() else {
        return Ok(HttpResponse::BadRequest().body("No next index"));
    };
    let quote_request = match body.into_inner() {
        QuoteBody::Simple(simple_quote) => bln_client::types::QuoteRequest {
            amount_msat: simple_quote.amount_msat,
            payee: simple_quote.payee,
        },
        QuoteBody::Bolt11(s) => {
            let Ok(invoice) = bln_client::types::Invoice::try_from(s.as_str()) else {
                return Ok(HttpResponse::BadRequest().body("Bad invoice"));
            };
            bln_client::types::QuoteRequest {
                amount_msat: invoice.amount_msat,
                payee: invoice.payee_compressed,
            }
        }
    };
    let bln_quote = match data.bln().quote(quote_request.clone()).await {
        Ok(y) => y,
        Err(err) => {
            log::info!("ERR : {:?}", err);
            return Ok(HttpResponse::InternalServerError().body("BLN quote not available"));
        }
    };
    // FIXME :: we need to sort out the Tos
    let amount =
        fx.msat_to_lovelace(quote_request.amount_msat + bln_quote.fee_msat) + FEE_PLACEHOLDER + 1;
    // fx.msat_to_lovelace(quote_request.amount_msat + bln_quote.fee_msat) + data.info().fee + 1;
    let relative_timeout = (ADAPTOR_TIME_DELTA + bln_quote.relative_timeout).as_millis() as u64;
    let response_body = crate::models::QuoteResponse {
        index,
        amount,
        relative_timeout,
        routing_fee: bln_quote.fee_msat,
    };
    Ok(HttpResponse::Ok().json(response_body))
}

pub async fn pay(
    req: HttpRequest,
    data: Data,
    body: web::Json<PayBody>,
) -> Result<HttpResponse, HandlerError> {
    let Some(keytag) = req.extensions().get::<Keytag>().cloned() else {
        return Ok(HttpResponse::InternalServerError().body("Error: Middleware data not found."));
    };
    let fx = data.fx().read().await.clone();
    let body = body.into_inner();
    let locked = Locked::new(body.cheque_body, body.signature);
    let invoice = match bln_client::types::Invoice::try_from(&body.invoice) {
        Ok(inv) => inv,
        Err(_) => return Ok(HttpResponse::BadRequest().body("Bad invoice")),
    };
    let (key, tag) = keytag.split();
    if !locked.verify(&key, &tag) {
        return Ok(HttpResponse::BadRequest().body("Invalid cheque"));
    };
    if invoice.payment_hash != locked.lock().0 {
        return Ok(HttpResponse::BadRequest().body(format!(
            "provided lock's secret={} does not match invoice's payment_hash={}",
            hex::encode(locked.lock().0),
            hex::encode(invoice.payment_hash),
        )));
    }

    let effective_amount_msat = fx.lovelace_to_msat(locked.amount() - FEE_PLACEHOLDER);
    if effective_amount_msat < invoice.amount_msat {
        return Ok(HttpResponse::BadRequest().body(format!(
            "cheque does not cover payment: minimum required={}, effective amount={}",
            invoice.amount_msat, effective_amount_msat
        )));
    }
    let fee_limit = effective_amount_msat - invoice.amount_msat + 1;

    // The cheque timeout is in posix time.
    // We need to convert to a time delta.
    // And then the BLN handler can convert to (relative) blocks and then block height
    // ie absolute blocks.
    let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return Ok(HttpResponse::InternalServerError().body("System time not available"));
    };

    let relative_timeout = locked
        .timeout()
        .saturating_sub(now)
        .saturating_sub(ADAPTOR_TIME_DELTA.saturating_add(ADAPTOR_TIME_GRACE));

    if relative_timeout.is_zero() {
        let min_timeout = (now + ADAPTOR_TIME_DELTA + ADAPTOR_TIME_GRACE).as_secs();
        return Ok(HttpResponse::InternalServerError().body(format!(
            "timeout too soon: minimum acceptable timeout={min_timeout}, provided timeout={}",
            locked.timeout().as_secs(),
        )));
    };

    if let Err(err) = data.db().append_locked(&keytag, locked).await {
        return Ok(HttpResponse::BadRequest().body(format!("Error handling cheque: {}", err)));
    };
    let pay_request = bln_client::types::PayRequest {
        fee_limit,
        relative_timeout,
        invoice,
    };
    let pay_response = match data.bln().pay(pay_request).await {
        Ok(res) => res,
        Err(err) => return Ok(HttpResponse::BadRequest().body(format!("Routing Error: {}", err))),
    };
    let channel = if let Some(secret) = pay_response.secret {
        data.db().unlock(&keytag, Secret(secret)).await
    } else {
        match data.db().get_channel(&keytag).await {
            Ok(Some(c)) => Ok(c),
            Ok(None) => return Ok(HttpResponse::InternalServerError().body("Impossible")),
            Err(err) => Err(err),
        }
    };
    let channel = match channel {
        Ok(channel) => channel,
        Err(err) => {
            return Ok(HttpResponse::BadRequest().body(format!("Error handling secret: {}", err)));
        }
    };
    let Some(receipt) = channel.receipt() else {
        return Ok(HttpResponse::InternalServerError().body("Failure to recover receipt"));
    };
    let response_body = match receipt.squash_proposal() {
        Ok(Some(propose)) => SquashResponse::Incomplete(propose),
        Ok(None) => SquashResponse::Complete,
        Err(_) => {
            return Ok(HttpResponse::InternalServerError().body("Failed to resolve squash"));
        }
    };
    Ok(HttpResponse::Ok().json(response_body))
}
