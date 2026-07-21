use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use actix_web::{App, HttpRequest, HttpResponse, HttpServer, body::BoxBody, http::StatusCode, web};
use cardano_sdk::Credential;
use minicbor::Decode;
use problem_details::ProblemDetail;
use serde::Serialize;
use serde::de::DeserializeOwned;

use konduit_server::{Db, Never, State, handlers};

// ---------------------------------------------------------------------------
// Media negotiation
// ---------------------------------------------------------------------------

#[derive(Clone)]
enum MediaType {
    Cbor,
    Json,
}

fn media_from_accept(req: &HttpRequest) -> MediaType {
    match req.headers().get("accept").and_then(|v| v.to_str().ok()) {
        Some(v) if v.contains("application/json") => MediaType::Json,
        _ => MediaType::Cbor,
    }
}

fn success<T: Serialize + minicbor::Encode<()>>(v: &T, media: &MediaType) -> HttpResponse {
    match media {
        MediaType::Json => HttpResponse::Ok()
            .content_type("application/json")
            .body(serde_json::to_vec(v).unwrap()),
        MediaType::Cbor => HttpResponse::Ok()
            .content_type("application/cbor")
            .body(minicbor::to_vec(v).unwrap()),
    }
}

fn problem<E: ProblemDetail>(e: &E, media: &MediaType) -> HttpResponse {
    let status = StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    match media {
        MediaType::Json => HttpResponse::build(status)
            .content_type("application/problem+json")
            .body(e.to_json()),
        MediaType::Cbor => HttpResponse::build(status)
            .content_type("application/problem+cbor")
            .body(e.to_cbor()),
    }
}

// ---------------------------------------------------------------------------
// Outcome — the one return type
// ---------------------------------------------------------------------------

pub struct Outcome<T, E>(pub Result<Result<T, E>, crate::handlers::Error>);

impl<T, E> Outcome<T, E> {
    pub fn ok(v: T) -> Self {
        Outcome(Ok(Ok(v)))
    }
    pub fn reject(e: E) -> Self {
        Outcome(Ok(Err(e)))
    }
    pub fn fatal(e: crate::handlers::Error) -> Self {
        Outcome(Err(e))
    }
}

impl<T, E> From<Result<Result<T, E>, crate::handlers::Error>> for Outcome<T, E> {
    fn from(r: Result<Result<T, E>, crate::handlers::Error>) -> Self {
        Outcome(r)
    }
}

impl<T, E> actix_web::Responder for Outcome<T, E>
where
    T: Serialize + minicbor::Encode<()>,
    E: ProblemDetail + fmt::Debug,
{
    type Body = BoxBody;

    fn respond_to(self, req: &HttpRequest) -> HttpResponse<Self::Body> {
        let media = media_from_accept(req);
        match self.0 {
            Ok(Ok(v)) => success(&v, &media),
            Ok(Err(e)) => problem(&e, &media),
            Err(e) => {
                log::error!("{e:?}");
                HttpResponse::InternalServerError().finish()
            }
        }
    }
}

// ---------------------------------------------------------------------------
// MediaBody extractor — CBOR first, then JSON
// ---------------------------------------------------------------------------

pub struct MediaBody<T>(pub T);

impl<T: DeserializeOwned + for<'a> Decode<'a, ()>> actix_web::FromRequest for MediaBody<T> {
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, payload: &mut actix_web::dev::Payload) -> Self::Future {
        let bytes = web::Bytes::from_request(req, payload);
        Box::pin(async move {
            let bytes = bytes.await?;
            let value = minicbor::decode(&bytes)
                .or_else(|_| serde_json::from_slice(&bytes))
                .map_err(actix_web::error::ErrorBadRequest)?;
            Ok(MediaBody(value))
        })
    }
}

// ---------------------------------------------------------------------------
// AuthHeaders extractor
// ---------------------------------------------------------------------------

/// We handle no auth cases downstream so that we can return our custom error type
/// consistently.
pub struct AuthHeader(pub Option<String>);

impl actix_web::FromRequest for AuthHeader {
    type Error = actix_web::Error;
    type Future = std::future::Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut actix_web::dev::Payload) -> Self::Future {
        let value = req
            .headers()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .map(|v| v.to_owned());
        std::future::ready(Ok(AuthHeader(value)))
    }
}

fn parse_auth(
    raw: Option<&str>,
) -> Result<konduit_wire::reg::cobbl3::Credential, konduit_wire::auth::AuthError> {
    let raw = raw.ok_or(konduit_wire::auth::AuthError::None)?;
    let (scheme, payload) = raw
        .split_once(' ')
        .ok_or(konduit_wire::auth::AuthError::Invalid)?;
    if scheme != konduit_wire::reg::cobbl3::SCHEME {
        return Err(konduit_wire::auth::AuthError::Invalid);
    }
    payload
        .parse::<konduit_wire::reg::cobbl3::Credential>()
        .map_err(|_| konduit_wire::auth::AuthError::Invalid)
}

fn extract_auth(
    auth: &AuthHeader,
) -> Result<konduit_wire::reg::cobbl3::Credential, konduit_wire::auth::AuthError> {
    parse_auth(auth.0.as_deref())
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn info(state: web::Data<State>) -> Outcome<konduit_wire::info::Response, Never> {
    Outcome::ok(handlers::info(&state))
}

async fn reg(
    body: MediaBody<konduit_wire::reg::cobbl3::Body>,
    state: web::Data<State>,
) -> Outcome<konduit_wire::reg::cobbl3::Response, konduit_wire::reg::cobbl3::Error> {
    Outcome(handlers::reg(body.0, &state))
}

// async fn squash(
//     header: AuthHeader,
//     body: MediaBody<konduit_wire::auth::squash::Body>,
//     state: web::Data<State>,
// ) -> Outcome<konduit_wire::auth::squash::Response, konduit_wire::auth::squash::Error> {
//     Outcome(handlers::auth::squash(header, body.0, &state))
// }

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let state = web::Data::new(State::new(
        Arc::new(COBBL3_KEY.clone().into()),
        Arc::new(Db::open("/tmp/konduit.db").expect("db open")),
        Arc::new(sample_response()),
    ));

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .route("/info", web::get().to(info))
            .route("/reg", web::post().to(reg))
            .service(
                web::scope("/auth"), //        .route("/squash", web::post().to(squash))
            )
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

// ---------------------------------------------------------------------------
// Constants / samples
// ---------------------------------------------------------------------------

pub const COBBL3_KEY: [u8; 32] = [0; 32];

fn sample_response() -> konduit_wire::info::Response {
    konduit_wire::info::Response {
        tos: konduit_wire::info::TosInfo {
            flat_fee: 1_000_000,
        },
        channel_parameters: konduit_wire::info::ChannelParameters {
            adaptor_key: konduit_data::VerifyingKey::from([0xab; 32]),
            close_period: konduit_data::Duration::from_secs(3600),
            tag_length: 8,
        },
        tx_help: konduit_wire::info::TxHelp {
            host_address: "addr_test1qqdxeujed4f77u82rslna9gtsrwnxqww8f3zxz8w4uz87vwy68rhqmmahtetcv2wcvsqe0ct9h6gmd8g5nsuw38rq4sqvh9dvw"
                .parse()
                .expect("valid address"),
            validator: cardano_sdk::Hash::from([0x01; 28]),
        },
    }
}
