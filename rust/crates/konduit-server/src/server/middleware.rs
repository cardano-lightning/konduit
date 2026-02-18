use actix_web::{
    Error, HttpMessage,
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
};
use konduit_data::Keytag;
use std::future::Future;
use std::future::{Ready, ready};
use std::pin::Pin;
use std::rc::Rc;

pub struct KeytagAuth {
    header_name: String,
}

impl KeytagAuth {
    pub fn new(header_name: &str) -> Self {
        Self {
            header_name: header_name.to_lowercase(),
        }
    }
}

impl<S, B> Transform<S, ServiceRequest> for KeytagAuth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = KeytagMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(KeytagMiddleware {
            service: Rc::new(service),
            header_name: self.header_name.clone(),
        }))
    }
}

pub struct KeytagMiddleware<S> {
    service: Rc<S>,
    header_name: String,
}

impl<S, B> Service<ServiceRequest> for KeytagMiddleware<S>
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
        let header_value = req
            .headers()
            .get(&self.header_name)
            .and_then(|v| v.to_str().ok());

        let mut keytag: Option<Keytag> = None;
        let mut error: Option<Error> = None;

        match header_value {
            Some(val) => {
                match hex::decode(val) {
                    Ok(bytes) => {
                        keytag = Some(Keytag(bytes));
                    }
                    Err(_) => {
                        // Invalid hex
                        error = Some(actix_web::error::ErrorForbidden(format!(
                            "invalid '{}' token format",
                            self.header_name
                        )));
                    }
                }
            }
            None => {
                error = Some(actix_web::error::ErrorForbidden(format!(
                    "missing '{}' header token",
                    self.header_name
                )));
            }
        }

        if let Some(err) = error {
            return Box::pin(async move { Err(err) });
        }

        if let Some(keytag) = keytag {
            req.extensions_mut().insert(keytag);
        }

        let srv = self.service.clone();

        Box::pin(async move {
            let res = srv.call(req).await?;
            Ok(res)
        })
    }
}
