use actix_web::{HttpResponse, ResponseError, body::BoxBody, http::StatusCode};

use super::error::Error;

impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        StatusCode::from_u16(crate::ApiError::status_code(self))
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
    }

    /// Defaults to CBOR — auth failures occur before content negotiation.
    fn error_response(&self) -> HttpResponse<BoxBody> {
        HttpResponse::build(self.status_code())
            .content_type("application/cbor")
            .body(minicbor::to_vec(self).expect("auth::hmac::Error is always encodable"))
    }
}
