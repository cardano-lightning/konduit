use std::future::{Ready, ready};

use actix_web::{
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    http::header,
    middleware::Next,
};
use actix_web::{
    FromRequest, HttpMessage, HttpRequest, HttpResponse, Responder, body::BoxBody, dev::Payload,
    error::ErrorInternalServerError,
};
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MediaType {
    Cbor,
    #[default]
    Json,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("unmediate: {0}")]
    Unmediate(String),
    #[error("backend: {0}")]
    Backend(String),
}

#[derive(Clone, Copy, Debug)]
pub struct Mediation {
    pub content: MediaType,
    pub accept: MediaType,
}

impl FromRequest for Mediation {
    type Error = actix_web::Error;
    type Future = Ready<Result<Self, Self::Error>>;
    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        ready(
            req.extensions().get::<Mediation>().copied().ok_or_else(|| {
                ErrorInternalServerError("content_negotiation middleware not mounted")
            }),
        )
    }
}

pub trait Unmediate: Sized {
    fn unmediate(content_type: MediaType, bytes: &[u8]) -> Result<Self, Error>;
}

impl<B> Unmediate for B
where
    B: serde::de::DeserializeOwned + for<'b> minicbor::Decode<'b, ()>,
{
    fn unmediate(content_type: MediaType, bytes: &[u8]) -> Result<Self, Error> {
        match content_type {
            MediaType::Json => serde_json::from_slice(bytes)
                .map_err(|e| Error::Unmediate(format!("invalid json body: {e}"))),
            MediaType::Cbor => minicbor::decode(bytes)
                .map_err(|e| Error::Unmediate(format!("invalid cbor body: {e}"))),
        }
    }
}

pub struct Mediate<T>(pub MediaType, pub T);

impl<T> Mediate<T> {
    pub fn ok<E>(self) -> Result<Mediate<T>, E> {
        Ok(self)
    }
}

impl<T: Serialize + minicbor::Encode<()>> Responder for Mediate<T> {
    type Body = BoxBody;
    fn respond_to(self, _req: &HttpRequest) -> HttpResponse<Self::Body> {
        match self.0 {
            MediaType::Json => HttpResponse::Ok().json(self.1),
            MediaType::Cbor => match minicbor::to_vec(&self.1) {
                Ok(bytes) => HttpResponse::Ok()
                    .content_type("application/cbor")
                    .body(bytes),
                Err(e) => {
                    HttpResponse::InternalServerError().body(format!("cbor encode error: {e}"))
                }
            },
        }
    }
}

fn parse_media_type(value: Option<&header::HeaderValue>) -> MediaType {
    value
        .and_then(|v| v.to_str().ok())
        .map(|s| {
            if s.contains("cbor") {
                MediaType::Cbor
            } else {
                MediaType::Json
            }
        })
        .unwrap_or_default() // Json
}

pub async fn content_negotiation(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    let content = parse_media_type(req.headers().get(header::CONTENT_TYPE));
    let accept = parse_media_type(req.headers().get(header::ACCEPT));

    req.extensions_mut().insert(Mediation { content, accept });

    next.call(req).await
}
