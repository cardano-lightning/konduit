use crate::Transport;
use core::future::Future;
use reqwest::Client;
use web_time::Duration;

pub struct ReqwestTransport {
    client: Client,
}

#[derive(Debug, thiserror::Error)]
pub enum ReqwestTransportError {
    #[error("reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error(transparent)]
    Http(#[from] http::Error),
    #[error("invalid HTTP status code: {0}")]
    InvalidStatus(#[from] http::status::InvalidStatusCode),
}

impl ReqwestTransport {
    pub fn new(timeout: Option<Duration>) -> Self {
        let mut builder = Client::builder();
        if let Some(dur) = timeout {
            builder = builder.timeout(dur);
        }
        Self {
            client: builder.build().unwrap(),
        }
    }
}

impl Transport for ReqwestTransport {
    type Error = ReqwestTransportError;

    fn transport(
        &self,
        req: http::Request<Vec<u8>>,
    ) -> impl Future<Output = Result<http::Response<Vec<u8>>, Self::Error>> + Send {
        let client = self.client.clone();
        async move {
            let (parts, body) = req.into_parts();
            let url = parts.uri.to_string();

            let mut builder = client.request(
                reqwest::Method::from_bytes(parts.method.as_str().as_bytes()).unwrap(),
                &url,
            );
            for (name, value) in &parts.headers {
                if let Ok(v) = value.to_str() {
                    builder = builder.header(name.as_str(), v);
                }
            }
            if !body.is_empty() {
                builder = builder.body(body);
            }

            let resp = builder.send().await?;
            let status = http::StatusCode::from_u16(resp.status().as_u16())?;
            let mut http_builder = http::Response::builder().status(status);
            for (name, value) in resp.headers() {
                if let Ok(v) = value.to_str() {
                    http_builder = http_builder.header(name.as_str(), v);
                }
            }
            let bytes = resp.bytes().await?.to_vec();
            Ok(http_builder.body(bytes)?)
        }
    }
}
