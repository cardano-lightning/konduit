use actix_web::{
    Error, HttpMessage,
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    http::header::HeaderName,
};
use cardano_sdk::VerificationKey;
use konduit_api::auth::pop;
use konduit_data::Keytag;
use std::{
    future::{Future, Ready, ready},
    pin::Pin,
    rc::Rc,
};

static HEADER_KEYTAG: HeaderName = HeaderName::from_static(pop::HEADER_KEYTAG);
static HEADER_SIGNATURE: HeaderName = HeaderName::from_static(pop::HEADER_SIGNATURE);

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
