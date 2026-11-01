use crate::server::{
    self,
    auth::AuthKeytag,
    data,
    mediation::{self, Mediate, Mediation, Unmediate},
};
use actix_web::{HttpResponse, ResponseError, http::StatusCode, web};
use konduit_tmp::{AdaptorInfo, PayOutcome, Quote, Receipt, SquashProposal, SquashStatus, TxHelp};
use std::ops::Deref;

type Data = web::Data<server::Data>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("mediation: {0}")]
    Mediation(#[from] mediation::Error),
    #[error("data: {0}")]
    Data(#[from] data::Error),
}

impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        match self {
            Error::Mediation(mediation::Error::Unmediate(_)) => StatusCode::BAD_REQUEST,
            Error::Mediation(mediation::Error::Backend(_)) => StatusCode::INTERNAL_SERVER_ERROR,
            // A problem states its own status; this only turns the
            // number into actix's type.
            Error::Data(data::Error::Problem { status, .. }) => {
                StatusCode::from_u16(*status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
            }
            Error::Data(data::Error::NoRail(_)) => StatusCode::NOT_FOUND,
            Error::Data(_) => StatusCode::BAD_REQUEST,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let status = self.status_code();
        if let Error::Data(data::Error::Problem { slug, detail, .. }) = self {
            return HttpResponse::build(status).json(serde_json::json!({
                "type": slug,
                "status": status.as_u16(),
                "detail": detail,
            }));
        }
        if status.is_server_error() {
            // don't leak internal details to the client, but keep them for yourself
            tracing::error!("request failed: {self}");
            HttpResponse::build(status).body("internal server error")
        } else {
            HttpResponse::build(status).body(self.to_string())
        }
    }
}

pub async fn info(mediation: Mediation, data: Data) -> Result<Mediate<AdaptorInfo<TxHelp>>, Error> {
    Ok(Mediate(mediation.accept, data.info().deref().clone()))
}

pub async fn fx(mediation: Mediation, data: Data) -> Result<Mediate<fx_client::State>, Error> {
    Ok(Mediate(mediation.accept, data.fx().read().await.clone()))
}

pub async fn payme(
    mediation: Mediation,
    data: Data,
    body: web::Bytes,
) -> Result<Mediate<konduit_tmp::Payme>, Error> {
    let request = Unmediate::unmediate(mediation.content, &body)?;
    Ok(Mediate(mediation.accept, data.payme(request)?))
}

/// `POST /pay/{rail}/quote`
pub async fn quote(
    mediation: Mediation,
    keytag: AuthKeytag,
    rail: web::Path<String>,
    data: Data,
    body: web::Bytes,
) -> Result<Mediate<Quote>, Error> {
    let request = Unmediate::unmediate(mediation.content, &body)?;
    let quote = data
        .quote(&keytag, &rail, request, mediation.content)
        .await?;
    Ok(Mediate(mediation.accept, quote))
}

/// `POST /pay/{rail}/pay`
pub async fn pay(
    mediation: Mediation,
    keytag: AuthKeytag,
    rail: web::Path<String>,
    data: Data,
    body: web::Bytes,
) -> Result<Mediate<PayOutcome>, Error> {
    let request = Unmediate::unmediate(mediation.content, &body)?;
    let outcome = data.pay(&keytag, &rail, request, mediation.content).await?;
    Ok(Mediate(mediation.accept, outcome))
}

pub async fn show(_data: Data) -> Result<HttpResponse, Error> {
    todo!()
    // log::info!("SHOW");
    // let keys = data.db().keys()?;
    // let results = keys
    //     .iter()
    //     .map(|x| data.db().get(x))
    //     .collect::<Result<Vec<_>, _>>()?;

    // Ok(HttpResponse::Ok().json(results))
}

/// Retrieve the latest receipt from the adaptor standpoint. This can be used by the consumer
/// to recover its own state without "fear":
///
/// - the squash is signed by their key, so necessarily originated from them.
/// - the adaptor is free to send an earlier receipt, which is only to the advantage of the
///   consumer for they will owe the adaptor *less* money. In practice, the adaptor has no
///   incentives to do that.
pub async fn receipt(
    mediation: Mediation,
    keytag: AuthKeytag,
    data: Data,
) -> Result<Mediate<Option<Receipt>>, Error> {
    Ok(Mediate(mediation.accept, data.receipt(&keytag)?))
}

pub async fn squash_proposal(
    mediation: Mediation,
    keytag: AuthKeytag,
    data: Data,
) -> Result<Mediate<SquashProposal>, Error> {
    Ok(Mediate(mediation.accept, data.squash_proposal(&keytag)?))
}

pub async fn squash_status(
    mediation: Mediation,
    keytag: AuthKeytag,
    data: Data,
) -> Result<Mediate<SquashStatus>, Error> {
    Ok(Mediate(mediation.accept, data.squash_status(&keytag)?))
}

pub async fn squash(
    mediation: Mediation,
    keytag: AuthKeytag,
    data: Data,
    body: web::Bytes,
    // ) -> Result<Mediate<()>, Error> {
) -> Result<Mediate<SquashStatus>, Error> {
    data.squash(&keytag, Unmediate::unmediate(mediation.content, &body)?)?;
    squash_status(mediation, keytag, data).await
}
