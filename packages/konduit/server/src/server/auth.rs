//! FIXME :: this will be proper auth
use actix_web::{
    Error, FromRequest, HttpMessage, HttpRequest,
    body::MessageBody,
    dev::{Payload, ServiceRequest, ServiceResponse},
    error::ErrorForbidden,
    middleware::Next,
};
use konduit_tmp::Keytag;
use std::future::{Ready, ready};
use std::ops::Deref;
use std::str::FromStr;

const KEYTAG_HEADER: &str = "KONDUIT";

pub async fn no_auth<B: MessageBody + 'static>(
    req: ServiceRequest,
    next: Next<B>,
) -> Result<ServiceResponse<B>, Error> {
    let header = req
        .headers()
        .get(KEYTAG_HEADER)
        .ok_or_else(|| ErrorForbidden(format!("missing '{KEYTAG_HEADER}' header token")))?
        .to_str()
        .map_err(|_| ErrorForbidden(format!("invalid '{KEYTAG_HEADER}' token format")))?;

    let keytag = Keytag::from_str(header)
        .map_err(|_| ErrorForbidden(format!("invalid '{KEYTAG_HEADER}' token format")))?;

    req.extensions_mut().insert(keytag);
    next.call(req).await
}

/// Local newtype wrapping `konduit_tmp::Keytag` so we can implement `FromRequest`
/// on it (the orphan rule blocks implementing it directly on the foreign `Keytag`).
///
/// Pulls the `Keytag` stashed into request extensions by the `no_auth` middleware.
/// Any route using this extractor MUST be mounted behind that middleware, or
/// extraction will fail with 403.
#[derive(Debug, Clone)]
pub struct AuthKeytag(pub Keytag);

impl Deref for AuthKeytag {
    type Target = Keytag;

    fn deref(&self) -> &Keytag {
        &self.0
    }
}

impl From<AuthKeytag> for Keytag {
    fn from(auth: AuthKeytag) -> Keytag {
        auth.0
    }
}

impl FromRequest for AuthKeytag {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let result = req
            .extensions()
            .get::<Keytag>()
            .cloned()
            .map(AuthKeytag)
            .ok_or_else(|| {
                ErrorForbidden(format!(
                    "missing '{KEYTAG_HEADER}' context; is `no_auth` middleware mounted on this scope?"
                ))
            });
        ready(result)
    }
}
