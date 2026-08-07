use crate::server::{
    self,
    auth::AuthKeytag,
    data,
    mediation::{self, Mediate, Mediation, Unmediate},
};
use actix_web::{HttpResponse, ResponseError, http::StatusCode, web};
use konduit_data::Locked;
use konduit_tmp::{AdaptorInfo, Quote, Receipt, SquashProposal, TxHelp};
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
        todo!()
    }

    fn error_response(&self) -> HttpResponse {
        todo!()
    }
}

pub async fn info(mediation: Mediation, data: Data) -> Result<Mediate<AdaptorInfo<TxHelp>>, Error> {
    Ok(Mediate(mediation.accept, data.info().deref().clone()))
}

pub async fn fx(mediation: Mediation, data: Data) -> Result<Mediate<fx_client::State>, Error> {
    Ok(Mediate(mediation.accept, data.fx().read().await.clone()))
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

pub async fn squash(
    mediation: Mediation,
    keytag: AuthKeytag,
    data: Data,
    body: web::Bytes,
) -> Result<Mediate<()>, Error> {
    Ok(Mediate(
        mediation.accept,
        data.squash(&keytag, Unmediate::unmediate(mediation.content, &body)?)?,
    ))
}

pub async fn quote(
    mediation: Mediation,
    keytag: AuthKeytag,
    data: Data,
    body: web::Bytes,
) -> Result<Mediate<Quote>, Error> {
    Ok(Mediate(
        mediation.accept,
        data.quote(&keytag, Unmediate::unmediate(mediation.content, &body)?)
            .await?,
    ))
}

// FIXME :: Remove the glue required here for historical reasons
pub async fn pay(
    mediation: Mediation,
    keytag: AuthKeytag,
    data: Data,
    body: web::Bytes,
) -> Result<Mediate<SquashProposal>, Error> {
    let b = konduit_tmp::PayBody::unmediate(mediation.content, &body)?;
    let locked = Locked::new(b.cheque_body, b.signature);
    let body = data::PayBody {
        locked,
        invoice: b.invoice,
    };
    let _ = data.pay(&keytag, body).await?;
    // FIXME : The return type here has diverged!!
    squash_proposal(mediation, keytag, data).await
}
