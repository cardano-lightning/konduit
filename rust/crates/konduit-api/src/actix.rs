use actix_web::{
    HttpRequest, HttpResponse, Responder, body::BoxBody, http::StatusCode, http::header,
};
use minicbor::Encode;
use serde::Serialize;

use crate::ApiError;

// ---------------------------------------------------------------------------
// Format negotiation
// ---------------------------------------------------------------------------

enum Format {
    Cbor,
    Json,
}

fn negotiate(req: &HttpRequest) -> Format {
    let accept = req
        .headers()
        .get(header::ACCEPT)
        .and_then(|h| h.to_str().ok());

    if let Some(val) = accept {
        if val.contains("application/cbor") {
            return Format::Cbor;
        }
        if val.contains("application/json") {
            return Format::Json;
        }
    }

    // Fallback: mirror the request Content-Type (symmetric API behaviour).
    let content_type = req
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|h| h.to_str().ok());

    if let Some(val) = content_type {
        if val.contains("application/json") {
            return Format::Json;
        }
    }

    Format::Cbor
}

fn encode_response<T>(format: &Format, status: StatusCode, val: &T) -> HttpResponse<BoxBody>
where
    T: Serialize + Encode<()>,
{
    match format {
        Format::Cbor => HttpResponse::build(status)
            .content_type("application/cbor")
            .body(minicbor::to_vec(val).expect("wire type is always CBOR-encodable")),
        Format::Json => HttpResponse::build(status).json(val),
    }
}

// ---------------------------------------------------------------------------
// ApiResult
// ---------------------------------------------------------------------------

/// Handler return type that negotiates the response format (CBOR or JSON)
/// from the request's `Accept` / `Content-Type` headers.
///
/// Convert any `Result<T, E>` with `into()`, then return from an actix handler:
///
/// ```ignore
/// pub async fn my_handler(req: HttpRequest, ...) -> ApiResult<MyResponse, MyError> {
///     inner(&req).await.into()
/// }
///
/// async fn inner(req: &HttpRequest) -> Result<MyResponse, MyError> {
///     // ? propagation works normally here
///     Ok(response)
/// }
/// ```
pub struct ApiResult<T, E>(Result<T, E>);

impl<T, E> From<Result<T, E>> for ApiResult<T, E> {
    fn from(r: Result<T, E>) -> Self {
        Self(r)
    }
}

impl<T, E> Responder for ApiResult<T, E>
where
    T: Serialize + Encode<()>,
    E: Serialize + Encode<()> + ApiError,
{
    type Body = BoxBody;

    fn respond_to(self, req: &HttpRequest) -> HttpResponse<Self::Body> {
        let format = negotiate(req);
        match self.0 {
            Ok(val) => encode_response(&format, StatusCode::OK, &val),
            Err(err) => {
                let status = StatusCode::from_u16(err.status_code())
                    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
                encode_response(&format, status, &err)
            }
        }
    }
}
