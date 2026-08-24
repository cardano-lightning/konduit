//! FIXME :: this will be proper auth
use actix_web::{
    Error, FromRequest, HttpMessage, HttpRequest,
    body::MessageBody,
    dev::{Payload, Service, ServiceRequest, ServiceResponse, Transform},
    error::ErrorForbidden,
    middleware::Next,
};
use konduit_tmp::Keytag;
use std::{
    future::{Future, Ready, ready},
    ops::Deref,
    pin::Pin,
    rc::Rc,
    str::FromStr,
};

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

pub struct LeaseAuth {
    check_url: String,
    client: reqwest::Client,
}

impl LeaseAuth {
    pub fn new(check_url: String) -> Self {
        Self {
            check_url,
            client: reqwest::Client::new(),
        }
    }
}

impl<S, B> Transform<S, ServiceRequest> for LeaseAuth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = LeaseMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(LeaseMiddleware {
            service: Rc::new(service),
            check_url: self.check_url.clone(),
            client: self.client.clone(),
        }))
    }
}

pub struct LeaseMiddleware<S> {
    service: Rc<S>,
    check_url: String,
    client: reqwest::Client,
}

impl<S, B> Service<ServiceRequest> for LeaseMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    actix_web::dev::forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let lease = req
            .headers()
            .get("FERRET-SESSION")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let client = self.client.clone();
        let check_url = self.check_url.clone();
        let service = self.service.clone();

        Box::pin(async move {
            let lease = lease
                .ok_or_else(|| actix_web::error::ErrorUnauthorized("missing FERRET-SESSION lease"))?;
            let valid = client
                .get(check_url)
                .header("FERRET-SESSION", lease)
                .send()
                .await
                .map_err(|_| {
                    actix_web::error::ErrorServiceUnavailable("lease verification unavailable")
                })?;
            if !valid.status().is_success() {
                return Err(actix_web::error::ErrorUnauthorized(
                    "stale FERRET-SESSION lease",
                ));
            }
            service.call(req).await
        })
    }
}
