use anyhow::anyhow;
use gloo_net::http::{Request, RequestBuilder, Response};
use gloo_timers::callback::Timeout;
use web_sys::{AbortController, AbortSignal};
use web_time::Duration;

pub struct HttpClient {
    base_url: String,
    http_timeout: Duration,
}

impl HttpClient {
    pub fn new(url: &str) -> Self {
        Self::with_timeout(url, Duration::from_secs(10))
    }

    pub fn with_timeout(url: &str, http_timeout: Duration) -> Self {
        Self {
            base_url: url.strip_suffix('/').unwrap_or(url).to_string(),
            http_timeout,
        }
    }

    fn mk_abort_on_timeout(timeout: &Duration) -> anyhow::Result<(AbortSignal, Timeout)> {
        let controller =
            AbortController::new().map_err(|_| anyhow!("failed to create AbortController"))?;
        let signal = controller.signal();
        let timeout_ms: u32 = timeout
            .as_millis()
            .try_into()
            .map_err(|_| anyhow!("timeout duration too large"))?;
        let timeout_controller = controller.clone();
        let timeout_handle = Timeout::new(timeout_ms, move || {
            timeout_controller.abort();
            log::warn!("aborted request due to timeout after {}ms", timeout_ms);
        });
        Ok((signal, timeout_handle))
    }

    async fn send(&self, request: Request) -> anyhow::Result<Vec<u8>> {
        let method = request.method();
        let url = request.url();
        let title = format!("{method} {url}");

        let response = request.send().await.map_err(|e| {
            log::error!("{title} failed: {e:?}");
            anyhow!(e)
        })?;

        Self::check_status(&title, &response).await?;

        response
            .binary()
            .await
            .map_err(|e| anyhow!("failed to read response body: {e}"))
    }

    async fn check_status(title: &str, response: &Response) -> anyhow::Result<()> {
        if !response.ok() {
            return Err(anyhow!(
                "{title} failed (status={}): {:?}",
                response.status(),
                response
                    .text()
                    .await
                    .unwrap_or_else(|_| "unable to decode response body".to_string()),
            ));
        }
        Ok(())
    }
}

impl http_client::HttpClient for HttpClient {
    type Error = anyhow::Error;

    fn base_url(&self) -> &str {
        &self.base_url
    }

    async fn request(
        &self,
        method: &http::Method,
        path: &str,
        headers: &[(&str, &str)],
        body: Option<&[u8]>,
    ) -> anyhow::Result<Vec<u8>> {
        let (abort_signal, timeout_handle) = Self::mk_abort_on_timeout(&self.http_timeout)?;

        let url = format!("{}{}", self.base_url, path);

        let mut req = RequestBuilder::new(&url).method(method.clone());

        req = headers.iter().fold(req, |req, (k, v)| req.header(k, v));
        req = req.abort_signal(Some(&abort_signal));

        let request = match body {
            Some(bytes) => {
                let body = js_sys::Uint8Array::from(bytes);
                req.body(body)?
            }
            None => req.build()?,
        };

        let result = self.send(request).await;
        timeout_handle.cancel();
        result
    }
}
