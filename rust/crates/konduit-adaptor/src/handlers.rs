<<<<<<< HEAD
use crate::config::AppState;
use crate::db::DbError;
use crate::models::{PayBody, QuoteBody, QuoteResponse, Receipt, SquashBody, UnlockedCheque};
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError, web};
=======
use crate::app_state::AppState;
use crate::cbor::decode_from_cbor;
use crate::db::DbError;
use crate::models::{QuoteBody, TipBody};
use actix_web::http::StatusCode;
use actix_web::{HttpMessage, HttpRequest, HttpResponse, ResponseError, web};
use cardano_tx_builder::cbor;
use konduit_data::{Keytag, Squash};
>>>>>>> e3cb13e (Updates to konduit data.)

impl ResponseError for DbError {
    fn status_code(&self) -> StatusCode {
        match self {
            DbError::Sled(_) => StatusCode::INTERNAL_SERVER_ERROR,
            DbError::Serde(_) => StatusCode::INTERNAL_SERVER_ERROR,
            DbError::TaskJoin(_) => StatusCode::INTERNAL_SERVER_ERROR,
            DbError::NotFound(_) => StatusCode::NOT_FOUND,
            DbError::InvalidData(_) => StatusCode::BAD_REQUEST,
        }
    }

    fn error_response(&self) -> HttpResponse {
        log::error!("Handler Error: {}", self);
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

<<<<<<< HEAD
pub async fn constants(data: web::Data<AppState>) -> Result<HttpResponse, DbError> {
    log::info!("GET /constants");
    let constants = data.db.get_constants().await?;
    Ok(HttpResponse::Ok().json(constants))
}

pub async fn quote(
    data: web::Data<AppState>,
    body: web::Json<QuoteBody>,
) -> Result<HttpResponse, DbError> {
    log::info!("POST /quote: {:?}", body);
    // FIXME :: Not yet implemented
    data.db.put_quote_request(&body).await?;
    let stub_response = QuoteResponse {
        amount: 100000,
        timeout: 3600000,
        lock: [1; 32],
        recipient: [2; 33],
        amount_msat: 99000,
        payment_addr: [3; 32],
        routing_fee: 1000,
    };
    data.db
        .put_quote_response(&body.consumer_key, &body.tag, &stub_response)
        .await?;

    Ok(HttpResponse::Ok().json(stub_response))
}

pub async fn pay(
    data: web::Data<AppState>,
    body: web::Json<PayBody>,
) -> Result<HttpResponse, DbError> {
    log::info!("POST /pay: {:?}", body);
    // FIXME :: Not yet implemented
    data.db.put_pay_request(&body).await?;
    let stub_receipt = Receipt {
        squash_body: vec![1, 2, 3],
        signature: [4; 64],
        unlockeds: vec![UnlockedCheque {
            cheque_body: vec![5, 6, 7],
            signature: [8; 64],
            secret: vec![9, 10, 11],
        }],
        expire: None,
    };
    data.db
        .put_receipt(&body.consumer_key, &body.tag, &stub_receipt)
        .await?;

    Ok(HttpResponse::Ok().json(stub_receipt))
}

pub async fn squash(
    data: web::Data<AppState>,
    body: web::Json<SquashBody>,
) -> Result<HttpResponse, DbError> {
    log::info!("POST /squash: {:?}", body);
    // FIXME :: Not yet implemented
    data.db.put_squash_request(&body).await?;
    match data.db.get_receipt(&body.consumer_key, &body.tag).await {
        Ok(receipt) => {
            // 200 OK - but this does not squash latest receipt
            log::warn!(
                "Squash request for channel that already has a receipt. Returning existing."
            );
            Ok(HttpResponse::Ok().json(receipt))
        }
        Err(DbError::NotFound(_)) => {
            // 202 Accepted - this squashes latest
            log::info!("Squash request accepted for channel.");
            Ok(HttpResponse::Accepted().finish())
        }
        Err(e) => Err(e), // Propagate other errors
    }
}
=======
pub async fn info(data: web::Data<AppState>) -> Result<HttpResponse, DbError> {
    log::info!("INFO");
    Ok(HttpResponse::Ok().json(&data.info))
}

pub async fn fx(data: web::Data<AppState>) -> Result<HttpResponse, DbError> {
    log::info!("FX");
    let data_guard = data.fx.read().await.clone();
    Ok(HttpResponse::Ok().json(&data_guard))
}

pub async fn tip(
    data: web::Data<AppState>,
    body: web::Json<TipBody>,
) -> Result<HttpResponse, DbError> {
    log::info!("TIP");
    let results = data.db.sync_tip(body.into_inner()).await?;
    Ok(HttpResponse::Ok().json(results))
}

pub async fn show(data: web::Data<AppState>) -> Result<HttpResponse, DbError> {
    log::info!("SHOW");
    let results = data.db.show().await?;
    Ok(HttpResponse::Ok().json(results))
}

pub async fn quote(
    req: HttpRequest,
    data: web::Data<AppState>,
    body: web::Json<QuoteBody>,
) -> Result<HttpResponse, DbError> {
    let extensions = req.extensions();
    let Some(keytag) = extensions.get::<Keytag>() else {
        return Ok(HttpResponse::InternalServerError().body("Error: Middleware data not found."));
    };
    let Some(fx) = data.fx.read().await.clone() else {
        return Ok(HttpResponse::InternalServerError().body("Error: Fx unavailable"));
    };
    let l2_channel = data.db.get_channel(keytag).await?;
    log::info!("L2_CHANNEL {:?}", l2_channel);
    if l2_channel.available() == 0 {
        return Ok(HttpResponse::InternalServerError().body("Error: Fx unavailable"));
    };
    let bln_quote = data.bln.quote(body.invoice.clone()).await.unwrap();

    log::info!("{:?}", bln_quote);
    let response_body = format!(
        "Hello from the handler!\nGot data from middleware: {:?}",
        keytag
    );
    Ok(HttpResponse::Ok().body(response_body))
}

pub async fn squash(
    req: HttpRequest,
    data: web::Data<AppState>,
    body: web::Bytes,
) -> Result<HttpResponse, DbError> {
    let extensions = req.extensions();
    let Some(keytag) = extensions.get::<Keytag>() else {
        return Ok(HttpResponse::InternalServerError().body("Error: Middleware data not found."));
    };
    let squash: Squash = decode_from_cbor(body.as_ref()).unwrap();
    let l2_channel = data.db.squash(keytag.clone(), squash).await?;
    // log::info!("L2_CHANNEL {:?}", l2_channel);
    // if l2_channel.available() == 0 {
    //     return Ok(HttpResponse::InternalServerError().body("Error: Fx unavailable"));
    // };
    // let bln_quote = data.bln.quote(body.invoice.clone()).await.unwrap();

    // log::info!("{:?}", bln_quote);
    let response_body = format!(
        "Hello from the handler!\nGot data from middleware: {:?}",
        keytag
    );
    Ok(HttpResponse::Ok().body(response_body))
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
>>>>>>> e3cb13e (Updates to konduit data.)
