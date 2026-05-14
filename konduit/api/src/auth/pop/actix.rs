use actix_web::{
    FromRequest, HttpRequest, HttpResponse, ResponseError, body::BoxBody, dev::Payload,
    http::StatusCode, http::header::HeaderName,
};
use futures::future::{Ready, ready};

use super::common;
use super::error::Error;

pub static HEADER_KEYTAG: HeaderName = HeaderName::from_static(common::HEADER_KEYTAG);
pub static HEADER_SIGNATURE: HeaderName = HeaderName::from_static(common::HEADER_SIGNATURE);

/// Defaults to CBOR — auth failures occur before content negotiation is possible.
impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        StatusCode::UNAUTHORIZED
    }

    fn error_response(&self) -> HttpResponse<BoxBody> {
        HttpResponse::build(self.status_code())
            .content_type("application/cbor")
            .body(minicbor::to_vec(self).expect("auth::pop::Error is always encodable"))
    }
}

impl FromRequest for common::Headers {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let result = (|| {
            let keytag = req
                .headers()
                .get(&HEADER_KEYTAG)
                .and_then(|v| v.to_str().ok())
                .ok_or(Error::MissingKeytag)?;

            let signature = req
                .headers()
                .get(&HEADER_SIGNATURE)
                .and_then(|v| v.to_str().ok())
                .ok_or(Error::MissingSignature)?;

            Ok(super::Headers {
                keytag: keytag.parse().map_err(|_| Error::BadKeytag)?,
                signature: signature.parse().map_err(|_| Error::BadSignature)?,
            })
        })();

        ready(result)
    }
}
