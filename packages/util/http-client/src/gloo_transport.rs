use crate::HttpTransport;
use crate::prelude::*;
use core::future::Future;

use futures::{FutureExt as _, select};
use gloo_net::http::RequestBuilder;
use gloo_timers::future::TimeoutFuture;
use js_sys::Uint8Array;
use send_wrapper::SendWrapper;
use web_time::Duration;

/// HTTP [`Transport`] backed by [`gloo_net`]. Suitable for `wasm32` targets.
///
/// ## `Send` bridging
///
/// [`SendWrapper`] makes the future `Send` by asserting single-thread access
/// at runtime.  In `wasm32-unknown-unknown` there is only one thread, so the
/// assertion never fires.
pub struct GlooTransport {
    timeout: Option<Duration>,
}

#[derive(Debug, thiserror::Error)]
pub enum GlooTransportError {
    #[error("request timed out")]
    Timeout,
    #[error("timeout duration too large to fit in u32 milliseconds")]
    TimeoutOverflow,
    #[error(transparent)]
    GlooNet(#[from] gloo_net::Error),
    #[error(transparent)]
    Http(#[from] http::Error),
    #[error("invalid HTTP status code: {0}")]
    InvalidStatus(#[from] http::status::InvalidStatusCode),
}

impl GlooTransport {
    pub fn new(timeout: Option<Duration>) -> Self {
        Self { timeout }
    }
}

impl HttpTransport for GlooTransport {
    type Error = GlooTransportError;

    fn transport(
        &self,
        req: http::Request<Vec<u8>>,
    ) -> impl Future<Output = Result<http::Response<Vec<u8>>, Self::Error>> + Send {
        // SendWrapper is Send+Sync unconditionally; it panics on cross-thread
        // access, which can't happen on the single-threaded wasm32 executor.
        SendWrapper::new(execute_gloo(req, self.timeout))
    }
}

async fn execute_gloo(
    req: http::Request<Vec<u8>>,
    timeout: Option<Duration>,
) -> Result<http::Response<Vec<u8>>, GlooTransportError> {
    let (parts, body) = req.into_parts();
    let url = parts.uri.to_string();

    let mut builder = RequestBuilder::new(&url).method(parts.method);
    for (name, value) in &parts.headers {
        if let Ok(v) = value.to_str() {
            builder = builder.header(name.as_str(), v);
        }
    }
    let gloo_req = if body.is_empty() {
        builder.build()?
    } else {
        builder.body(Uint8Array::from(body.as_slice()))?
    };

    // Race the fetch against an optional timer — no AbortController needed.
    let resp = match timeout {
        None => gloo_req.send().await?,
        Some(dur) => {
            let millis =
                u32::try_from(dur.as_millis()).map_err(|_| GlooTransportError::TimeoutOverflow)?;
            select! {
                r = gloo_req.send().fuse() => r?,
                _ = TimeoutFuture::new(millis).fuse() => {
                    return Err(GlooTransportError::Timeout)
                }
            }
        }
    };

    let status = http::StatusCode::from_u16(resp.status())?;
    let mut http_builder = http::Response::builder().status(status);
    for (name, value) in resp.headers().entries() {
        http_builder = http_builder.header(&name, &value);
    }
    Ok(http_builder.body(resp.binary().await?)?)
}
