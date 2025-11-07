use std::time::Duration;

use crate::app_state::AppState;
use crate::cbor::decode_from_cbor;
use crate::db::DbError;
use crate::models::{IncompleteSquashResponse, QuoteBody, SquashResponse, TipBody};
use crate::{Invoice, bln};
use actix_web::http::StatusCode;
use actix_web::{HttpMessage, HttpRequest, HttpResponse, ResponseError, web};
use konduit_data::{Keytag, Squash};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum HandlerError {
    #[error("Db : {0}")]
    Db(String),
    #[error("String : {0}")]
    Logic(String),
}

impl<T> From<DbError<T>> for HandlerError
where
    T: std::fmt::Display + std::fmt::Debug,
{
    fn from(value: DbError<T>) -> Self {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Info {
    #[serde(with = "hex")]
    pub adaptor_key: [u8; 32],
    pub close_period: u64,
    pub fee: u64,
    pub max_tag_length: usize,
    #[serde(with = "hex")]
    pub deployer_vkey: [u8; 32],
    #[serde(with = "hex")]
    pub script_hash: [u8; 28],
}

pub async fn info(data: web::Data<AppState>) -> HttpResponse {
    let info = Info {
        adaptor_key: data.info.adaptor_key.into(),
        close_period: data.info.close_period.into(),
        fee: data.info.fee,
        max_tag_length: data.info.max_tag_length,
        deployer_vkey: data.info.deployer_vkey.into(),
        script_hash: data.info.script_hash.into(),
    };
    HttpResponse::Ok().json(&info)
}

pub async fn fx(data: web::Data<AppState>) -> HttpResponse {
    log::info!("FX");
    let data_guard = data.fx.read().await.clone();
    HttpResponse::Ok().json(&data_guard)
}

pub async fn tip(
    data: web::Data<AppState>,
    body: web::Json<TipBody>,
) -> Result<HttpResponse, HandlerError> {
    log::info!("TIP");
    let results = data.db.update_l1s(body.into_inner()).await?;
    Ok(HttpResponse::Ok().json(results))
}

pub async fn show(data: web::Data<AppState>) -> Result<HttpResponse, HandlerError> {
    log::info!("SHOW");
    let results = data.db.get_all().await?;
    Ok(HttpResponse::Ok().json(results))
}

pub async fn squash(
    req: HttpRequest,
    data: web::Data<AppState>,
    body: web::Bytes,
) -> Result<HttpResponse, HandlerError> {
    let extensions = req.extensions();
    let Some(keytag) = extensions.get::<Keytag>() else {
        return Ok(HttpResponse::InternalServerError().body("Error: Middleware data not found."));
    };
    let Ok(squash): Result<Squash, _> = decode_from_cbor(body.as_ref()) else {
        return Ok(HttpResponse::BadRequest().body("Cannot decode squash"));
    };
    let (key, tag) = keytag.split();
    log::info!("squash {:?}", squash.verify(&key, &tag));
    let l2_channel = data.db.update_squash(keytag.clone(), squash).await?;
    let Some(mixed_receipt) = l2_channel.mixed_receipt else {
        return Ok(HttpResponse::InternalServerError().body("Impossible result"));
    };
    // FIXME :: This should be moved to a single method eg `squashable` on mixed receipt
    let response_body = if mixed_receipt.unlockeds().len() > 0 {
        // FIXME :: Should include possible expire
        SquashResponse::Incomplete(IncompleteSquashResponse {
            mixed_receipt,
            expire: None,
        })
    } else {
        SquashResponse::Complete
    };
    Ok(HttpResponse::Ok().json(response_body))
}

pub async fn quote(
    req: HttpRequest,
    data: web::Data<AppState>,
    body: web::Json<QuoteBody>,
) -> Result<HttpResponse, HandlerError> {
    let extensions = req.extensions();
    let Some(keytag) = extensions.get::<Keytag>() else {
        return Ok(HttpResponse::InternalServerError().body("Error: Middleware data not found."));
    };
    let Some(fx) = data.fx.read().await.clone() else {
        log::info!("FX : {:?}", data.fx);
        return Ok(HttpResponse::InternalServerError().body("Error: Fx unavailable"));
    };
    let Some(l2_channel) = data.db.get_channel(keytag).await? else {
        return Ok(HttpResponse::BadRequest().body("No channel found"));
    };
    if l2_channel.available() == 0 {
        return Ok(HttpResponse::BadRequest().body("No channel funds"));
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
    let Ok(bln_quote) = data.bln.quote(quote_request.clone()).await else {
        return Ok(HttpResponse::InternalServerError().body("BLN quote not available"));
    };

    // TODO :: MOVE TO CONFIG
    // This is ~ the same as the default on bitcoin
    let adaptor_margin = Duration::from_secs(40 * 10 * 60);

    log::info!("{:?}", bln_quote);
    let amount =
        fx.msat_to_lovelace(quote_request.amount_msat + bln_quote.fee_msat) + data.info.fee;
    let timeout = (adaptor_margin + bln_quote.estimated_timeout).as_millis() as u64;
    let response_body = crate::models::QuoteResponse {
        amount: amount,
        timeout: timeout,
        routing_fee: bln_quote.fee_msat,
    };
    Ok(HttpResponse::Ok().json(response_body))
}
