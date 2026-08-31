use crate::wire::auth::{HEADER, Keytag};
use actix_web::{
    Error as ActixError, HttpMessage,
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    error::ErrorBadRequest,
    middleware::Next,
};

pub async fn auth(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, ActixError> {
    let raw = req
        .headers()
        .get(HEADER)
        .ok_or_else(|| ErrorBadRequest("missing konduit header"))?
        .to_str()
        .map_err(|_| ErrorBadRequest("konduit header not ascii"))?;

    let keytag: Keytag = raw.parse().map_err(ErrorBadRequest)?;
    req.extensions_mut().insert(keytag);
    next.call(req).await
}

#[cfg(feature = "mock")]
impl actix_web::FromRequest for Keytag {
    type Error = actix_web::Error;
    type Future = std::future::Ready<Result<Self, Self::Error>>;
    fn from_request(req: &actix_web::HttpRequest, _: &mut actix_web::dev::Payload) -> Self::Future {
        std::future::ready(
            req.extensions()
                .get::<Keytag>()
                .cloned()
                .ok_or_else(|| actix_web::error::ErrorInternalServerError("keytag missing")),
        )
    }
}
