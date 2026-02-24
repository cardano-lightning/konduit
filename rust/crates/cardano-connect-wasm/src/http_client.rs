use anyhow::anyhow;
use gloo_net::http::{Request, Response};
use gloo_timers::callback::Timeout;
use wasm_bindgen::prelude::*;
use web_sys::{AbortController, AbortSignal};
use web_time::Duration;

pub struct HttpClient {
    pub base_url: String,
    pub http_timeout: Duration,
}

impl HttpClient {
    pub fn new(base_url: String, http_timeout: Duration) -> Self {
        Self {
            base_url,
            http_timeout,
        }
    }

    fn mk_abort_on_timeout(timeout: &Duration) -> anyhow::Result<(AbortSignal, Timeout)> {
        let controller =
            AbortController::new().map_err(|_| anyhow!("Failed to create AbortController"))?;

        let signal: AbortSignal = controller.signal();

        let timeout_ms: u32 = timeout
            .as_millis()
            .try_into()
            .map_err(|_| anyhow!("timeout duration too large"))?;

        let timeout_controller = controller.clone(); // Clone for move into closure

        let timeout_handle = Timeout::new(timeout_ms, move || {
            timeout_controller.abort();
            log::warn!("Aborted request due to timeout after {}ms", timeout_ms);
        });

        anyhow::Ok((signal, timeout_handle))
    }

    async fn send<T: serde::de::DeserializeOwned>(&self, request: Request) -> anyhow::Result<T> {
        let method = request.method();
        let url = request.url();
        let title = format!("{method} {url}");
        let title_str = title.as_str();

        let response = request.send().await.map_err(|e| {
            log::error!("{title_str} failed: {e:?}");
            anyhow!(e)
        })?;

        Self::handle_non_success(title_str, &response).await?;

        response
            .json()
            .await
            .map_err(|e| anyhow!("invalid JSON response from backend: {e}"))
    }

    async fn handle_non_success(title: &str, response: &Response) -> anyhow::Result<()> {
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

    pub async fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> anyhow::Result<T> {
        self.get_with_headers(path, &[]).await
    }

    pub async fn get_with_headers<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        headers: &[(&str, &str)],
    ) -> anyhow::Result<T> {
        let (abort_on_timeout, timeout_handle) = Self::mk_abort_on_timeout(&self.http_timeout)?;
        let request = headers
            .iter()
            .fold(
                Request::get(&format!("{}{path}", self.base_url)),
                |req, (key, value)| req.header(key, value),
            )
            .abort_signal(Some(&abort_on_timeout))
            .build()?;
        let result = self.send::<T>(request).await;
        timeout_handle.cancel();
        result
    }

    pub async fn post_with_headers<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        headers: &[(&str, &str)],
        body: impl Into<JsValue>,
    ) -> anyhow::Result<T> {
        let body = js_sys::JSON::stringify(&body.into())
            .map_err(|e| anyhow!("failed to serialize request body: {:?}", e))?;
        let (abort_on_timeout, timeout_handle) = Self::mk_abort_on_timeout(&self.http_timeout)?;
        let request = headers
            .iter()
            .fold(
                Request::post(&format!("{}{path}", self.base_url)),
                |req, (key, value)| req.header(key, value),
            )
            .abort_signal(Some(&abort_on_timeout))
            .body(body)?;
        let result = self.send::<T>(request).await;
        timeout_handle.cancel();
        result
    }

    pub async fn post<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: impl Into<JsValue>,
    ) -> anyhow::Result<T> {
        self.post_with_headers(path, &[], body).await
    }
}
