use actix_web::{HttpResponse, ResponseError, http::StatusCode};
use std::fmt;

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    body: &'static str,
}

impl ApiError {
    pub fn bad_request() -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            body: "bad request",
        }
    }

    pub fn conflict() -> Self {
        Self {
            status: StatusCode::CONFLICT,
            body: "conflict",
        }
    }

    pub fn not_found() -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            body: "not found",
        }
    }

    pub fn too_many() -> Self {
        Self {
            status: StatusCode::TOO_MANY_REQUESTS,
            body: "too many requests",
        }
    }

    pub fn payload() -> Self {
        Self {
            status: StatusCode::PAYLOAD_TOO_LARGE,
            body: "payload too large",
        }
    }

    pub fn unavailable() -> Self {
        Self {
            status: StatusCode::SERVICE_UNAVAILABLE,
            body: "unavailable",
        }
    }

    pub fn unexpected() -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            body: "unexpected error",
        }
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.body)
    }
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        self.status
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status)
            .content_type("text/plain; charset=utf-8")
            .body(self.body)
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(error: anyhow::Error) -> Self {
        log::warn!("dependency failure");
        let _ = error;
        Self::unavailable()
    }
}
