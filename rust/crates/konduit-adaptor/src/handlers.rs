use crate::app_state::AppState;
use crate::cbor::decode_from_cbor;
use crate::db::DbError;
use crate::models::{IncompleteSquashResponse, QuoteBody, SquashResponse, TipBody};
use crate::{Invoice, bln};
use actix_web::http::StatusCode;
use actix_web::{HttpMessage, HttpRequest, HttpResponse, ResponseError, web};
use konduit_data::{Keytag, Squash};

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

pub async fn info(data: web::Data<AppState>) -> HttpResponse {
    log::info!("INFO");
    HttpResponse::Ok().json(&data.info)
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
        return Ok(HttpResponse::InternalServerError().body("No funds"));
    };
    let quote_request = match body.into_inner() {
        QuoteBody::Bolt11(s) => {
            let Ok(invoice) = Invoice::try_from(s.as_str()) else {
                return Ok(HttpResponse::BadRequest().body("Bad invoice"));
            };
            bln::QuoteRequest::Bolt11(invoice)
        }
    };
    let bln_quote = data.bln.quote(quote_request).await.unwrap();

    log::info!("{:?}", bln_quote);
    let amount =
        ((bln_quote.amount_msat as f64) * fx.bitcoin / fx.ada * 1_000_000_000.0) as u64 + 1;
    let timeout = 0;
    let response_body = crate::models::QuoteResponse {
        amount: amount,
        timeout: timeout,
        lock: [0; 32],
        recipient: bln_quote.recipient,
        amount_msat: bln_quote.amount_msat,
        payment_secret: [0; 32],
        routing_fee: 0,
    };
    Ok(HttpResponse::Ok().json(response_body))
}

// let bln_quote = data.bln.quote(body.into_inner().invoice).await.unwrap();
// todo!()
//     // 1. Verify key_tag
//     let amount_available = match data.db.get_available(keytag).await {
//         Ok(amount) => amount,
//         Err(err) => todo!(),
//     };
//
//     // 2. Request quote from BLN
//     //
//     //
//     // 3. Get price data
//     //
//     //
//     //
//     // FIXME :: Not yet implemented
//     data.db.put_quote_request(&body).await?;
//     let stub_response = QuoteResponse {
//         amount: 100000,
//         timeout: 3600000,
//         lock: [1; 32],
//         recipient: [2; 33],
//         amount_msat: 99000,
//         payment_addr: [3; 32],
//         routing_fee: 1000,
//     };
//     data.db
//         .put_quote_response(&body.consumer_key, &body.tag, &stub_response)
//         .await?;
//
//     Ok(HttpResponse::Ok().json(stub_response))
//
// pub async fn pay(
//     data: web::Data<AppState>,
//     body: web::Json<PayBody>,
// ) -> Result<HttpResponse, DbError> {
//     log::info!("POST /pay: {:?}", body);
//     // FIXME :: Not yet implemented
//     data.db.put_pay_request(&body).await?;
//     let stub_receipt = Receipt {
//         squash_body: vec![1, 2, 3],
//         signature: [4; 64],
//         unlockeds: vec![UnlockedCheque {
//             cheque_body: vec![5, 6, 7],
//             signature: [8; 64],
//             secret: vec![9, 10, 11],
//         }],
//         expire: None,
//     };
//     data.db
//         .put_receipt(&body.consumer_key, &body.tag, &stub_receipt)
//         .await?;
//
//     Ok(HttpResponse::Ok().json(stub_receipt))
// }
//
// pub async fn squash(
//     data: web::Data<AppState>,
//     body: web::Json<SquashBody>,
// ) -> Result<HttpResponse, DbError> {
//     log::info!("POST /squash: {:?}", body);
//     // FIXME :: Not yet implemented
//     data.db.put_squash_request(&body).await?;
//     match data.db.get_receipt(&body.consumer_key, &body.tag).await {
//         Ok(receipt) => {
//             // 200 OK - but this does not squash latest receipt
//             log::warn!(
//                 "Squash request for channel that already has a receipt. Returning existing."
//             );
//             Ok(HttpResponse::Ok().json(receipt))
//         }
//         Err(DbError::NotFound(_)) => {
//             // 202 Accepted - this squashes latest
//             log::info!("Squash request accepted for channel.");
//             Ok(HttpResponse::Accepted().finish())
//         }
//         Err(e) => Err(e), // Propagate other errors
//     }
// }
