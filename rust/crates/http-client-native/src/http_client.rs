use anyhow::{Context, anyhow};
use reqwest::Method;
use std::ops::Deref;

pub struct HttpClient {
    base_url: String,
    http_client: reqwest::Client,
}

impl Deref for HttpClient {
    type Target = reqwest::Client;
    fn deref(&self) -> &Self::Target {
        &self.http_client
    }
}

impl HttpClient {
    pub fn new(url: &str) -> Self {
        Self {
            base_url: url.strip_suffix("/").unwrap_or(url).to_string(),
            http_client: reqwest::Client::new(),
        }
    }

    async fn request<T: serde::de::DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        headers: &[(&str, &str)],
        body: Option<Vec<u8>>,
    ) -> anyhow::Result<T> {
        let mut req = self
            .http_client
            .request(method, format!("{}{}", self.base_url, path));

        req = headers.iter().fold(req, |req, (k, v)| req.header(*k, *v));

        if let Some(bytes) = body {
            req = req.body(bytes);
        }

        let res = req.send().await?;
        let status = res.status();
        let body: String = res.text().await?;

        if !status.is_success() {
            return Err(anyhow!(
                "request to {} failed ({}): {}",
                path,
                status,
                &body
            ));
        }

        serde_json::from_str::<T>(&body).with_context(|| {
            format!(
                "failed to parse response for type {}",
                std::any::type_name::<T>(),
            )
        })
    }
}

impl http_client::HttpClient for HttpClient {
    type Error = anyhow::Error;

    fn to_json<V: serde::Serialize>(value: &V) -> Vec<u8> {
        serde_json::to_vec(value)
            .unwrap_or_else(|e| unreachable!("failed to serialised to vector? {e}"))
    }

    async fn get_with_headers<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        headers: &[(&str, &str)],
    ) -> anyhow::Result<T> {
        self.request(Method::GET, path, headers, None).await
    }

    async fn post_with_headers<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        headers: &[(&str, &str)],
        body: impl AsRef<[u8]>,
    ) -> anyhow::Result<T> {
        self.request(Method::POST, path, headers, Some(body.as_ref().to_vec()))
            .await
    }
}
