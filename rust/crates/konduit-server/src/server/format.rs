use actix_web::{HttpRequest, HttpResponse, Responder, http::header};
use konduit_data::api::media_format::MediaFormat;
use serde::Serialize;
use std::fmt::Display;

/// Infers the preferred media format from the request headers.
/// 1. Checks 'Accept' header for 'application/cbor' or 'application/json'.
/// 2. If missing/ambiguous, checks 'Content-Type' of the request.
/// 3. Defaults to Cbor.
pub fn from_request(req: &HttpRequest) -> MediaFormat {
    let accept = req
        .headers()
        .get(header::ACCEPT)
        .and_then(|h| h.to_str().ok());

    if let Some(val) = accept {
        if val.contains("application/cbor") {
            return MediaFormat::Cbor;
        }
        if val.contains("application/json") {
            return MediaFormat::Json;
        }
    }

    // Fallback: Infer from request Content-Type (symmetric API behavior)
    let content_type = req
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|h| h.to_str().ok());
    if let Some(val) = content_type {
        if val.contains("application/cbor") {
            return MediaFormat::Cbor;
        }
    }

    MediaFormat::Cbor
}
