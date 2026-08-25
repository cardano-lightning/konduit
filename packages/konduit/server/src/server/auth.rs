use crate::db;
// FIXME :: this will be proper auth
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
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
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
    db: Arc<db::Db>,
}

impl LeaseAuth {
    pub fn new(db: Arc<db::Db>) -> Self {
        Self { db }
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
            db: self.db.clone(),
        }))
    }
}

pub struct LeaseMiddleware<S> {
    service: Rc<S>,
    db: Arc<db::Db>,
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
        let token = req
            .headers()
            .get("FERRET-SESSION")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| {
                let mut token = [0; 32];
                hex::decode_to_slice(value, &mut token).ok().map(|_| token)
            });
        let wallet_key = req.extensions().get::<Keytag>().map(|keytag| {
            let (verification_key, _) = keytag.split();
            <[u8; 32]>::from(verification_key)
        });
        let db = self.db.clone();
        let service = self.service.clone();

        Box::pin(async move {
            let (Some(token), Some(wallet_key)) = (token, wallet_key) else {
                return Err(actix_web::error::ErrorUnauthorized(
                    "missing or malformed FERRET-SESSION lease",
                ));
            };
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(actix_web::error::ErrorInternalServerError)?
                .as_millis() as u64;
            match db.validate_lease(&wallet_key, &token, now) {
                Ok(true) => service.call(req).await,
                Ok(false) => Err(actix_web::error::ErrorUnauthorized(
                    "stale FERRET-SESSION lease",
                )),
                Err(error) => Err(actix_web::error::ErrorInternalServerError(error)),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{App, HttpResponse, http::StatusCode, middleware, test, web};
    use cardano_sdk::SigningKey;
    use konduit_data::Tag;
    use konduit_tmp::SessionClaimRequest;

    #[actix_web::test]
    async fn lease_auth_is_wallet_bound_and_receipt_is_unprotected() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let db = Arc::new(db::Db::open(file.path().to_str().unwrap()).unwrap());
        let wallet_a = SigningKey::from([1; 32]);
        let wallet_b = SigningKey::from([2; 32]);
        let tag = Tag::from(b"lease-test".as_slice());
        let keytag_a = Keytag::new(&wallet_a.to_verification_key(), &tag).to_string();
        let keytag_b = Keytag::new(&wallet_b.to_verification_key(), &tag).to_string();
        let claim = SessionClaimRequest::signed(&wallet_a, 1, [7; 32], [8; 32], 0);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let app = test::init_service(
            App::new().service(
                web::scope("/ch")
                    .wrap(middleware::from_fn(no_auth))
                    .route(
                        "/receipt",
                        web::get().to(|| async { HttpResponse::Ok().finish() }),
                    )
                    .service(
                        web::resource("/protected")
                            .wrap(LeaseAuth::new(db.clone()))
                            .route(web::post().to(|| async { HttpResponse::Ok().finish() })),
                    ),
            ),
        )
        .await;

        for request in [
            test::TestRequest::post()
                .uri("/ch/protected")
                .insert_header(("KONDUIT", keytag_a.clone()))
                .to_request(),
            test::TestRequest::post()
                .uri("/ch/protected")
                .insert_header(("KONDUIT", keytag_a.clone()))
                .insert_header(("FERRET-SESSION", "not-hex"))
                .to_request(),
        ] {
            let error = test::try_call_service(&app, request).await.unwrap_err();
            assert_eq!(
                error.as_response_error().status_code(),
                StatusCode::UNAUTHORIZED
            );
        }

        db.claim_lease(&claim, [3; 32], now - 1).unwrap();
        let error = test::try_call_service(
            &app,
            test::TestRequest::post()
                .uri("/ch/protected")
                .insert_header(("KONDUIT", keytag_a.clone()))
                .insert_header(("FERRET-SESSION", hex::encode([3; 32])))
                .to_request(),
        )
        .await
        .unwrap_err();
        assert_eq!(
            error.as_response_error().status_code(),
            StatusCode::UNAUTHORIZED
        );

        db.claim_lease(&claim, [4; 32], now + 60_000).unwrap();
        let error = test::try_call_service(
            &app,
            test::TestRequest::post()
                .uri("/ch/protected")
                .insert_header(("KONDUIT", keytag_b))
                .insert_header(("FERRET-SESSION", hex::encode([4; 32])))
                .to_request(),
        )
        .await
        .unwrap_err();
        assert_eq!(
            error.as_response_error().status_code(),
            StatusCode::UNAUTHORIZED
        );

        let response = test::call_service(
            &app,
            test::TestRequest::post()
                .uri("/ch/protected")
                .insert_header(("KONDUIT", keytag_a.clone()))
                .insert_header(("FERRET-SESSION", hex::encode([4; 32])))
                .to_request(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);

        db.claim_lease(&claim, [5; 32], now + 60_000).unwrap();
        let error = test::try_call_service(
            &app,
            test::TestRequest::post()
                .uri("/ch/protected")
                .insert_header(("KONDUIT", keytag_a.clone()))
                .insert_header(("FERRET-SESSION", hex::encode([4; 32])))
                .to_request(),
        )
        .await
        .unwrap_err();
        assert_eq!(
            error.as_response_error().status_code(),
            StatusCode::UNAUTHORIZED
        );

        let response = test::call_service(
            &app,
            test::TestRequest::get()
                .uri("/ch/receipt")
                .insert_header(("KONDUIT", keytag_a))
                .to_request(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
    }
}
