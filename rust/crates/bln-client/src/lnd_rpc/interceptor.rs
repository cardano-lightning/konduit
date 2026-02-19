use tonic::Status;
use tonic::metadata::MetadataValue;

use crate::macaroon::Macaroon;

/// A gRPC interceptor that injects the LND Macaroon into every request header.
#[derive(Clone)]
pub struct Interceptor {
    macaroon: Macaroon,
}

impl Interceptor {
    pub fn new(macaroon: Macaroon) -> Self {
        Self { macaroon: macaroon }
    }
}

impl tonic::service::Interceptor for Interceptor {
    fn call(&mut self, mut request: tonic::Request<()>) -> Result<tonic::Request<()>, Status> {
        let m = MetadataValue::try_from(&hex::encode(&self.macaroon))
            .map_err(|_| Status::invalid_argument("Invalid macaroon format"))?;
        request.metadata_mut().insert("macaroon", m);
        Ok(request)
    }
}
