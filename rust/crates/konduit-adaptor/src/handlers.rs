use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::{
    Invoice, State, bln,
    cbor::decode_from_cbor,
    db::DbError,
    fx, info,
    models::{IncompleteSquashResponse, PayBody, QuoteBody, SquashResponse, TipBody},
};
use actix_web::{HttpMessage, HttpRequest, HttpResponse, ResponseError, http::StatusCode, web};
use konduit_data::{Keytag, Secret, Squash};

// TODO :: MOVE TO CONFIG
/// This is ~ the same as the default on bitcoin: default (apparently) is 40 blocks
const ADAPTOR_TIME_DELTA: std::time::Duration = Duration::from_secs(40 * 10 * 60);
/// "Grace" is extra time between the "quoted" rel time and the time that might be allowed for in a
/// "pay"
const ADAPTOR_TIME_GRACE: std::time::Duration = Duration::from_secs(1 * 10 * 60);

#[derive(Debug, thiserror::Error)]
pub enum HandlerError {
    #[error("Db : {0}")]
    Db(String),
    #[error("String : {0}")]
    Logic(String),
}

impl From<DbError> for HandlerError {
    fn from(value: DbError) -> Self {
        match value {
            DbError::Backend(error) => HandlerError::Db(error.to_string()),
            DbError::Logic(error) => HandlerError::Logic(error.to_string()),
        }
    }
}

impl ResponseError for HandlerError {
    fn status_code(&self) -> StatusCode {
        match self {
            HandlerError::Db(_) => StatusCode::INTERNAL_SERVER_ERROR,
            HandlerError::Logic(_) => StatusCode::BAD_REQUEST,
        }
    }

    fn error_response(&self) -> HttpResponse {
        log::error!("Handler Error: {}", self);
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

pub async fn info(data: web::Data<State>) -> info::Info {
    (*data.info()).clone()
}

pub async fn fx(data: web::Data<State>) -> Result<fx::Fx, fx::Error> {
    data.fx()
        .read()
        .await
        .clone()
        .ok_or(fx::Error::Other("unavailable".to_string()))
}

pub async fn show(data: web::Data<State>) -> Result<HttpResponse, HandlerError> {
    log::info!("SHOW");
    let results = data.db().get_all().await?;
    Ok(HttpResponse::Ok().json(results))
}

pub async fn squash(
    req: HttpRequest,
    data: web::Data<State>,
    body: web::Bytes,
) -> Result<HttpResponse, HandlerError> {
    let Some(keytag) = req.extensions().get::<Keytag>().cloned() else {
        return Ok(HttpResponse::InternalServerError().body("Error: Middleware data not found."));
    };
    let Ok(squash): Result<Squash, _> = decode_from_cbor(body.as_ref()) else {
        return Ok(HttpResponse::BadRequest().body("Cannot decode squash"));
    };
    let (key, tag) = keytag.split();
    if !squash.verify(&key, &tag) {
        return Ok(HttpResponse::BadRequest().body("Invalid squash"));
    }
    let channel = data.db().update_squash(&keytag, squash).await?;
    let Some(receipt) = channel.receipt() else {
        return Ok(HttpResponse::InternalServerError().body("Impossible result"));
    };
    // FIXME :: This should be moved to a single method eg `squashable`
    let response_body = if !receipt.unlockeds().is_empty() {
        // FIXME :: Should include possible expire
        SquashResponse::Incomplete(IncompleteSquashResponse {
            receipt,
            expire: None,
        })
    } else {
        SquashResponse::Complete
    };
    Ok(HttpResponse::Ok().json(response_body))
}

pub async fn quote(
    req: HttpRequest,
    data: web::Data<State>,
    body: web::Json<QuoteBody>,
) -> Result<HttpResponse, HandlerError> {
    let Some(keytag) = req.extensions().get::<Keytag>().cloned() else {
        return Ok(HttpResponse::InternalServerError().body("Error: Middleware data not found."));
    };
    let Some(fx) = data.fx().read().await.clone() else {
        log::info!("FX : {:?}", data.fx());
        return Ok(HttpResponse::InternalServerError().body("Error: Fx unavailable"));
    };
    let Some(channel) = data.db().get_channel(&keytag).await? else {
        return Ok(HttpResponse::BadRequest().body("No channel found"));
    };
    let Ok(capacity) = channel.capacity() else {
        return Ok(HttpResponse::BadRequest().body("No capacity"));
    };
    let quote_request = match body.into_inner() {
        QuoteBody::Simple(simple_quote) => bln::QuoteRequest {
            amount_msat: simple_quote.amount_msat,
            payee: simple_quote.payee,
        },
        QuoteBody::Bolt11(s) => {
            let Ok(invoice) = Invoice::try_from(s.as_str()) else {
                return Ok(HttpResponse::BadRequest().body("Bad invoice"));
            };
            bln::QuoteRequest {
                amount_msat: invoice.amount_msat,
                payee: invoice.payee_compressed,
            }
        }
    };
    let Ok(bln_quote) = data.bln().quote(quote_request.clone()).await else {
        return Ok(HttpResponse::InternalServerError().body("BLN quote not available"));
    };
    let amount =
        fx.msat_to_lovelace(quote_request.amount_msat + bln_quote.fee_msat) + data.info().fee + 1;
    let relative_timeout = (ADAPTOR_TIME_DELTA + bln_quote.relative_timeout).as_millis() as u64;
    let response_body = crate::models::QuoteResponse {
        amount,
        relative_timeout,
        routing_fee: bln_quote.fee_msat,
    };
    Ok(HttpResponse::Ok().json(response_body))
}

pub async fn pay(
    req: HttpRequest,
    data: web::Data<State>,
    body: web::Json<PayBody>,
) -> Result<HttpResponse, HandlerError> {
    let Some(keytag) = req.extensions().get::<Keytag>().cloned() else {
        return Ok(HttpResponse::InternalServerError().body("Error: Middleware data not found."));
    };
    let Some(fx) = data.fx().read().await.clone() else {
        log::info!("FX : {:?}", data.fx());
        return Ok(HttpResponse::InternalServerError().body("Error: Fx unavailable"));
    };
    let pay_body = body.into_inner();
    let invoice = match Invoice::try_from(&pay_body.invoice) {
        Ok(inv) => inv,
        Err(_) => return Ok(HttpResponse::BadRequest().body("Bad invoice")),
    };
    let (key, tag) = keytag.split();
    if !pay_body.cheque.verify(&key, &tag) {
        return Ok(HttpResponse::BadRequest().body("Invalid cheque"));
    };
    let effective_amount_msat =
        fx.lovelace_to_msat(pay_body.cheque.cheque_body.amount - data.info().fee);
    if effective_amount_msat < invoice.amount_msat {
        return Ok(HttpResponse::BadRequest().body("Cheque does not cover payment"));
    }
    let fee_limit = effective_amount_msat - invoice.amount_msat + 1;

    // The cheque timeout is in posix time.
    // We need to convert to a time delta.
    // And then the BLN handler can convert to (relative) blocks and then block height
    // ie absolute blocks.
    let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return Ok(HttpResponse::InternalServerError().body("System time not available"));
    };
    let relative_timeout = pay_body
        .cheque
        .cheque_body
        .timeout
        .saturating_sub(now)
        .saturating_sub(ADAPTOR_TIME_DELTA.saturating_add(ADAPTOR_TIME_GRACE));

    if relative_timeout.is_zero() {
        return Ok(HttpResponse::InternalServerError().body("Timeout too soon"));
    };

    // let payment_hash = pay_body.cheque.cheque_body.lock.0.clone();
    if let Err(err) = data.db.insert_cheque(&keytag, pay_body.cheque).await {
        return Ok(HttpResponse::BadRequest().body(format!("Error handling cheque: {}", err)));
    };
    let pay_request = bln::PayRequest {
        fee_limit,
        relative_timeout,
        invoice: invoice,
    };

    let pay_response = match data.bln.pay(pay_request).await {
        Ok(res) => res,
        Err(err) => return Ok(HttpResponse::BadRequest().body(format!("Routing Error: {}", err))),
    };
    let channel = match data.db.unlock(&keytag, Secret(pay_response.secret)).await {
        Ok(channel) => channel,
        Err(err) => {
            return Ok(HttpResponse::BadRequest()
                .body(format!("Error handling secret: {}", err.to_string())));
        }
    };
    let Some(receipt) = channel.receipt() else {
        return Ok(HttpResponse::InternalServerError().body("Logic failure"));
    };
    Ok(HttpResponse::Ok().json(receipt))
}
