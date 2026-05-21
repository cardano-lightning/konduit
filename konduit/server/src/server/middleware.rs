use actix_web::{
    Error, HttpMessage,
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    http::header::HeaderName,
};
use cardano_sdk::VerificationKey;
use konduit_api::auth::{hmac, pop};
use konduit_data::Keytag;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{
    future::{Future, Ready, ready},
    pin::Pin,
    rc::Rc,
};

static HEADER_KEYTAG: HeaderName = HeaderName::from_static(pop::HEADER_KEYTAG);
static HEADER_SIGNATURE: HeaderName = HeaderName::from_static(pop::HEADER_SIGNATURE);
static HEADER_HMAC_TOKEN: HeaderName = HeaderName::from_static(hmac::HEADER_TOKEN);

pub struct Pop {
    server_key: VerificationKey,
}

impl Pop {
    pub fn new(server_key: VerificationKey) -> Self {
        Self { server_key }
    }
}

impl<S, B> Transform<S, ServiceRequest> for Pop
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = PopAuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(PopAuthMiddleware {
            service: Rc::new(service),
            server_key: self.server_key,
        }))
    }
}

pub struct PopAuthMiddleware<S> {
    service: Rc<S>,
    server_key: VerificationKey,
}

impl<S, B> Service<ServiceRequest> for PopAuthMiddleware<S>
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
        let server_key = self.server_key;

        let result: Result<Keytag, Error> = (|| {
            let keytag_str = req
                .headers()
                .get(&HEADER_KEYTAG)
                .and_then(|v| v.to_str().ok())
                .ok_or_else(|| actix_web::error::ErrorUnauthorized("missing konduit-keytag"))?;

            let signature_str = req
                .headers()
                .get(&HEADER_SIGNATURE)
                .and_then(|v| v.to_str().ok())
                .ok_or_else(|| actix_web::error::ErrorUnauthorized("missing konduit-signature"))?;

            let keytag: Keytag = keytag_str
                .parse()
                .map_err(|_| actix_web::error::ErrorUnauthorized("malformed keytag"))?;

            let signature: cardano_sdk::Signature = signature_str
                .parse()
                .map_err(|_| actix_web::error::ErrorUnauthorized("malformed signature"))?;

            let payload = pop::to_bytes(&server_key, &keytag);
            let (client_key, _tag) = keytag.split();
            if !client_key.verify(&payload, &signature) {
                return Err(actix_web::error::ErrorUnauthorized("invalid pop signature"));
            }

            Ok(keytag)
        })();

        match result {
            Ok(keytag) => {
                req.extensions_mut().insert(keytag);
                let srv = self.service.clone();
                Box::pin(async move { srv.call(req).await })
            }
            Err(err) => Box::pin(async move { Err(err) }),
        }
    }
}

// ---------------------------------------------------------------------------
// HMAC-BLAKE3 session token middleware
// ---------------------------------------------------------------------------

/// Transform factory.  Holds the server's HMAC key and public key so the
/// middleware can verify tokens without any I/O.
pub struct HmacToken {
    hmac_key: [u8; 32],
    server_key: VerificationKey,
}

impl HmacToken {
    pub fn new(hmac_key: [u8; 32], server_key: VerificationKey) -> Self {
        Self {
            hmac_key,
            server_key,
        }
    }
}

impl<S, B> Transform<S, ServiceRequest> for HmacToken
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = HmacTokenMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(HmacTokenMiddleware {
            service: Rc::new(service),
            hmac_key: self.hmac_key,
            server_key: self.server_key,
        }))
    }
}

pub struct HmacTokenMiddleware<S> {
    service: Rc<S>,
    hmac_key: [u8; 32],
    server_key: VerificationKey,
}

impl<S, B> Service<ServiceRequest> for HmacTokenMiddleware<S>
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
        let hmac_key = self.hmac_key;
        let server_key = self.server_key;

        let result: Result<Keytag, Error> = (|| {
            let header_val = req
                .headers()
                .get(&HEADER_HMAC_TOKEN)
                .and_then(|v| v.to_str().ok())
                .ok_or_else(|| {
                    actix_web::error::ErrorUnauthorized(
                        minicbor::to_vec(&hmac::Error::MissingToken).expect("always encodable"),
                    )
                })?;

            let token = hmac::token_from_header(header_val).map_err(|e| {
                actix_web::error::ErrorUnauthorized(minicbor::to_vec(&e).expect("always encodable"))
            })?;

            let now_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

            hmac::verify_token(&hmac_key, &server_key, &token, now_ms).map_err(|e| {
                actix_web::error::ErrorUnauthorized(minicbor::to_vec(&e).expect("always encodable"))
            })?;

            Ok(token.keytag)
        })();

        match result {
            Ok(keytag) => {
                req.extensions_mut().insert(keytag);
                let srv = self.service.clone();
                Box::pin(async move { srv.call(req).await })
            }
            Err(err) => Box::pin(async move { Err(err) }),
        }
    }
}
