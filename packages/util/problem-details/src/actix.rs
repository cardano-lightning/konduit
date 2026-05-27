use actix_web::{
    HttpResponse, ResponseError,
    body::BoxBody,
    http::{StatusCode, header},
};
use std::fmt;

use crate::{CONTENT_TYPE_CBOR, ProblemDetail};

/// Returns `(status_code, content_type, body)` for any [`ProblemDetail`].
/// Use this when integrating with a framework not covered by a feature flag.
pub fn into_parts(p: &impl ProblemDetail) -> (u16, &'static str, Vec<u8>) {
    (p.http_status(), CONTENT_TYPE_CBOR, p.to_cbor())
}

// ── Actix-web ─────────────────────────────────────────────────────────────────

/// Wraps any [`ProblemDetail`] as an actix-web [`ResponseError`].
///
/// # Example
/// ```rust,ignore
/// async fn handler() -> Result<HttpResponse, Problem<Error>> {
///     Err(Problem(Error::Unauthorized))
/// }
///
/// // With runtime context:
/// async fn handler() -> Result<HttpResponse, Problem<WithDetail<Error>>> {
///     use problem_details_server::ProblemDetailExt;
///     Err(Problem(Error::Unauthorized.with_detail(format!("clock skew: {skew}s"))))
/// }
/// ```
pub struct Problem<E>(pub E);

impl<E: ProblemDetail + fmt::Debug> fmt::Debug for Problem<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl<E: ProblemDetail + fmt::Debug> fmt::Display for Problem<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.0.title(), self.0.http_status())
    }
}

impl<E: ProblemDetail + fmt::Debug> ResponseError for Problem<E> {
    fn status_code(&self) -> StatusCode {
        StatusCode::from_u16(self.0.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
    }
    fn error_response(&self) -> HttpResponse<BoxBody> {
        HttpResponse::build(self.status_code())
            .insert_header((header::CONTENT_TYPE, CONTENT_TYPE_CBOR))
            .body(self.0.to_cbor())
    }
}

impl<E: ProblemDetail> From<E> for Problem<E> {
    fn from(e: E) -> Self {
        Problem(e)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{ProblemDetailBody, ProblemDetailExt};
    use actix_web::{
        App, HttpResponse, ResponseError, body::to_bytes, http::StatusCode, test, web,
    };
    use problem_details_derive::ProblemDetail;

    #[derive(Debug, ProblemDetail)]
    enum Error {
        #[problem(slug = "unauthorized", title = "Unauthorized", http_status = 401)]
        Unauthorized,
        #[problem(slug = "not-a-patron", title = "Not a Patron", http_status = 403)]
        NotPatron,
    }

    // ── Status code ───────────────────────────────────────────────────────────

    #[actix_web::test]
    async fn status_code_matches_http_status() {
        let err = Problem(Error::Unauthorized);
        let resp = err.error_response();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn status_code_matches_http_status_403() {
        let err = Problem(Error::NotPatron);
        let resp = err.error_response();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    // ── Content-Type ──────────────────────────────────────────────────────────

    #[actix_web::test]
    async fn response_content_type_is_problem_cbor() {
        use actix_web::ResponseError;
        let resp = Problem(Error::Unauthorized).error_response();
        let ct = resp
            .headers()
            .get("content-type")
            .expect("content-type header missing")
            .to_str()
            .unwrap();
        assert_eq!(ct, CONTENT_TYPE_CBOR);
    }

    // ── Body ──────────────────────────────────────────────────────────────────

    #[actix_web::test]
    async fn body_deserialises_to_problem_detail_body() {
        use actix_web::ResponseError;

        let resp = Problem(Error::NotPatron).error_response();
        let bytes = to_bytes(resp.into_body()).await.unwrap();
        let decoded: ProblemDetailBody = minicbor::decode(&bytes).expect("cbor decode failed");

        assert_eq!(decoded.status, 403);
        assert_eq!(decoded.title, "Not a Patron");
        assert!(decoded.r#type.ends_with("/not-a-patron"));
    }

    #[actix_web::test]
    async fn body_detail_absent_when_not_set() {
        use actix_web::ResponseError;

        let resp = Problem(Error::Unauthorized).error_response();
        let bytes = to_bytes(resp.into_body()).await.unwrap();
        let decoded: ProblemDetailBody = minicbor::decode(&bytes).expect("cbor decode failed");

        assert_eq!(decoded.detail, None);
    }

    // ── WithDetail ────────────────────────────────────────────────────────────

    #[actix_web::test]
    async fn with_detail_propagates_to_body() {
        use actix_web::ResponseError;

        let err = Problem(Error::Unauthorized.with_detail("clock skew was 12s"));
        let resp = err.error_response();
        let bytes = to_bytes(resp.into_body()).await.unwrap();
        let decoded: ProblemDetailBody = minicbor::decode(&bytes).expect("cbor decode failed");

        assert_eq!(decoded.status, 401);
        assert_eq!(decoded.detail, Some("clock skew was 12s".to_owned()));
    }

    // ── From impl ─────────────────────────────────────────────────────────────

    #[test]
    async fn from_impl_wraps_error() {
        let p: Problem<Error> = Error::Unauthorized.into();
        assert_eq!(p.0.slug(), "unauthorized");
    }

    // ── Display ───────────────────────────────────────────────────────────────

    #[test]
    async fn display_includes_title_and_status() {
        let p = Problem(Error::Unauthorized);
        assert_eq!(p.to_string(), "Unauthorized (401)");
    }

    // ── Integration: handler returns Problem as ResponseError ─────────────────

    #[actix_web::test]
    async fn handler_returning_problem_error_produces_correct_response() {
        async fn handler() -> Result<HttpResponse, Problem<Error>> {
            Err(Problem(Error::NotPatron))
        }

        let app = test::init_service(App::new().route("/", web::get().to(handler))).await;

        let req = test::TestRequest::get().uri("/").to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        let ct = resp
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(ct, CONTENT_TYPE_CBOR);

        let bytes = test::read_body(resp).await;
        let decoded: ProblemDetailBody = minicbor::decode(&bytes).expect("cbor decode failed");
        assert_eq!(decoded.title, "Not a Patron");
    }
}
