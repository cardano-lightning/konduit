use actix_web::{
    Error, FromRequest, HttpRequest, dev::Payload, error::ErrorBadRequest, http::header::HeaderName,
};
use futures::future::{Ready, ready};

use super::common;

pub static HEADER_KEYTAG: HeaderName = HeaderName::from_static(common::HEADER_KEYTAG);
pub static HEADER_SIGNATURE: HeaderName = HeaderName::from_static(common::HEADER_SIGNATURE);

/// Requires feature `actix`
impl FromRequest for common::Headers {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let result = (|| {
            let keytag = req
                .headers()
                .get(HEADER_KEYTAG)
                .and_then(|v| v.to_str().ok())
                .ok_or_else(|| ErrorBadRequest("missing Keytag"))?;

            let signature = req
                .headers()
                .get(HEADER_SIGNATURE)
                .and_then(|v| v.to_str().ok())
                .ok_or_else(|| ErrorBadRequest("missing Signature"))?;

            Ok(super::Headers {
                keytag: keytag.parse().map_err(ErrorBadRequest)?,
                signature: signature.parse().map_err(ErrorBadRequest)?,
            })
        })();

        ready(result)
    }
}
