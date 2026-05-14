use crate::db;
use actix_web::{HttpMessage, HttpRequest, web};
use konduit_api::{
    DepthBucket,
    actix::ApiResult,
    endpoints::{
        channel::{pay::quoted, quote::bolt11, squash, sync},
        info, version,
    },
};
use konduit_channel::Error as ChannelError;
use konduit_data::Keytag;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

type Data = web::Data<super::Data>;

// TODO :: MOVE TO CONFIG
/// Adaptor margin on top of the BLN-reported relative timeout.
const ADAPTOR_TIME_DELTA: Duration = Duration::from_secs(40 * 10 * 60);
/// Extra buffer between the quoted relative timeout and what the adaptor allows for in pay.
const QUOTE_PAY_TIME_MARGIN: Duration = Duration::from_secs(4 * 10 * 60);

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn get_keytag(req: &HttpRequest) -> Option<Keytag> {
    req.extensions().get::<Keytag>().cloned()
}

fn channel_err_to_sync(e: ChannelError) -> sync::Error {
    match e {
        ChannelError::Backing => sync::Error::Backing,
        _ => sync::Error::Backing,
    }
}

fn channel_err_to_squash(e: ChannelError) -> squash::Error {
    match e {
        ChannelError::Backing => squash::Error::Backing,
        ChannelError::Input => squash::Error::Invalid,
        _ => squash::Error::Invalid,
    }
}

fn channel_err_to_quoted(e: ChannelError) -> quoted::Error {
    match e {
        ChannelError::Backing => quoted::Error::Backing,
        ChannelError::Input => quoted::Error::Cheque,
        ChannelError::Funds => quoted::Error::Funds {
            available: 0,
            required: 0,
        },
        ChannelError::Capacity => quoted::Error::Capacity,
        ChannelError::Receipt => quoted::Error::Squash,
    }
}

fn channel_err_to_bolt11(e: ChannelError) -> bolt11::Error {
    match e {
        ChannelError::Backing => bolt11::Error::Backing,
        ChannelError::Receipt => bolt11::Error::Squash,
        ChannelError::Capacity => bolt11::Error::Capacity,
        ChannelError::Funds => bolt11::Error::Funds {
            available: 0,
            required: 0,
        },
        ChannelError::Input => bolt11::Error::Other("bad input".into()),
    }
}

fn db_err_to_sync(e: db::Error) -> sync::Error {
    log::error!("db error: {e}");
    sync::Error::Backing
}

fn db_err_to_squash(e: db::Error) -> squash::Error {
    log::error!("db error: {e}");
    squash::Error::Backing
}

fn db_err_to_quoted(e: db::Error) -> quoted::Error {
    log::error!("db error: {e}");
    quoted::Error::Backing
}

fn db_err_to_bolt11(e: db::Error) -> bolt11::Error {
    log::error!("db error: {e}");
    bolt11::Error::Other("db error".into())
}

// ---------------------------------------------------------------------------
// Unauthenticated endpoints
// ---------------------------------------------------------------------------

pub async fn version(_req: HttpRequest) -> HttpResponse {
    use actix_web::HttpResponse;
    use konduit_api::endpoints::version::{Response, SemVer, VcsHash};
    use std::collections::BTreeMap;

    let resp = Response {
        flavor: "default".into(),
        release: SemVer {
            major: 0,
            minor: 1,
            patch: 0,
        },
        protocol_hash: "unknown".into(),
        vcs_hash: VcsHash::Unknown,
        features: BTreeMap::new(),
        docs_base_url: None,
    };
    HttpResponse::Ok().json(resp)
}

pub async fn info(data: Data) -> actix_web::HttpResponse {
    actix_web::HttpResponse::Ok().json(data.info().as_ref())
}

// ---------------------------------------------------------------------------
// Channel endpoints — all require PoP auth (keytag in request extensions)
// ---------------------------------------------------------------------------

pub async fn sync(req: HttpRequest, data: Data) -> ApiResult<sync::Response, sync::Error> {
    inner_sync(&req, &data).await.into()
}

async fn inner_sync(req: &HttpRequest, data: &Data) -> Result<sync::Response, sync::Error> {
    let keytag = get_keytag(req).ok_or(sync::Error::Backing)?;

    let channel = data
        .db()
        .get_channel(&keytag)
        .await
        .map_err(db_err_to_sync)?
        .ok_or(sync::Error::Backing)?;

    let backing = channel.backing().best_live().map(|utxo| sync::BackingView {
        amount: utxo.amount(),
        // We don't have current block height in the handler yet.
        // Use Settled as a conservative default until indexer integration.
        bucket: DepthBucket::Settled,
    });

    let squash_proposal = channel
        .squash_proposal()
        .ok()
        .map(konduit_api::common::channel::SquashProposal::from);

    Ok(sync::Response {
        backing,
        squash_proposal,
    })
}

pub async fn squash(
    req: HttpRequest,
    data: Data,
    body: web::Bytes,
) -> ApiResult<squash::Response, squash::Error> {
    inner_squash(&req, &data, body).await.into()
}

async fn inner_squash(
    req: &HttpRequest,
    data: &Data,
    body: web::Bytes,
) -> Result<squash::Response, squash::Error> {
    let keytag = get_keytag(req).ok_or(squash::Error::Backing)?;

    let request: squash::Request = minicbor::decode(&body).map_err(|_| squash::Error::Invalid)?;

    data.db()
        .update_channel(&keytag, Box::new(move |ch| ch.apply_squash(request.squash)))
        .await
        .map_err(|e| match e {
            db::Error::Logic(db::LogicError::NoEntry(_)) => squash::Error::Backing,
            db::Error::Logic(db::LogicError::Channel(ce)) => channel_err_to_squash(ce),
            e => {
                log::error!("db error in squash: {e}");
                squash::Error::Backing
            }
        })?;

    Ok(squash::Response::Ok)
}

pub async fn pay_quoted(
    req: HttpRequest,
    data: Data,
    body: web::Bytes,
) -> ApiResult<quoted::Response, quoted::Error> {
    inner_pay_quoted(&req, &data, body).await.into()
}

async fn inner_pay_quoted(
    req: &HttpRequest,
    data: &Data,
    body: web::Bytes,
) -> Result<quoted::Response, quoted::Error> {
    let keytag = get_keytag(req).ok_or(quoted::Error::Backing)?;

    let request: quoted::Request = minicbor::decode(&body).map_err(|_| quoted::Error::Cheque)?;

    let locked = request.cheque.as_locked().ok_or(quoted::Error::Cheque)?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| quoted::Error::Backing)?;

    let cheque_timeout = locked.timeout();
    if cheque_timeout
        .saturating_sub(now)
        .saturating_sub(ADAPTOR_TIME_DELTA)
        .is_zero()
    {
        return Err(quoted::Error::Timeout {
            minimum_ms: (now + ADAPTOR_TIME_DELTA).as_millis() as u64,
        });
    }

    let _channel = data
        .db()
        .update_channel(&keytag, Box::new(move |ch| ch.apply_locked(locked.clone())))
        .await
        .map_err(|e| match e {
            db::Error::Logic(db::LogicError::NoEntry(_)) => quoted::Error::Backing,
            db::Error::Logic(db::LogicError::Channel(ce)) => channel_err_to_quoted(ce),
            e => {
                log::error!("db error in pay_quoted: {e}");
                quoted::Error::Backing
            }
        })?;

    // TODO: forward to BLN using the invoice.
    // Currently we accept the cheque and return Inflight;
    // the BLN payment routing will be added when the invoice
    // is included in the request or cached from the prior quote.
    Ok(quoted::Response::Inflight)
}

pub async fn quote_bolt11(
    req: HttpRequest,
    data: Data,
    body: web::Bytes,
) -> ApiResult<bolt11::Response, bolt11::Error> {
    inner_quote_bolt11(&req, &data, body).await.into()
}

async fn inner_quote_bolt11(
    req: &HttpRequest,
    data: &Data,
    body: web::Bytes,
) -> Result<bolt11::Response, bolt11::Error> {
    let keytag = get_keytag(req).ok_or(bolt11::Error::Backing)?;

    let request: bolt11::Request = minicbor::decode(&body).map_err(|_| bolt11::Error::Size)?;

    let invoice = &request.0;

    let channel = data
        .db()
        .get_channel(&keytag)
        .await
        .map_err(db_err_to_bolt11)?
        .ok_or(bolt11::Error::Backing)?;

    let index = channel.next_index().map_err(channel_err_to_bolt11)?;
    let available = channel.spendable().map_err(channel_err_to_bolt11)?;

    let fx = data.fx().read().await.clone();

    let bln_quote = data
        .bln()
        .quote(bln_client::types::QuoteRequest {
            amount_msat: invoice.amount_msat,
            payee: invoice.payee_compressed.serialize(),
            route_hints: invoice
                .private_route
                .iter()
                .cloned()
                .map(|pr| {
                    bln_client::types::RouteHint(
                        pr.0.iter()
                            .map(|h| bln_client::types::RouteHintHop::from(h.clone()))
                            .collect(),
                    )
                })
                .collect(),
        })
        .await
        .map_err(|_| bolt11::Error::Route)?;

    let flat_fee = data.info().tos.flat_fee;
    let amount = fx.msat_to_lovelace(invoice.amount_msat + bln_quote.fee_msat) + flat_fee;

    if amount > available {
        return Err(bolt11::Error::Funds {
            available,
            required: amount,
        });
    }

    let relative_timeout = (ADAPTOR_TIME_DELTA + QUOTE_PAY_TIME_MARGIN + bln_quote.relative_timeout)
        .as_millis() as u64;

    Ok(bolt11::Response {
        index,
        amount,
        relative_timeout,
        fee: bln_quote.fee_msat,
    })
}

// Shim until actix_web::HttpResponse is used in scope via `use`
use actix_web::HttpResponse;
