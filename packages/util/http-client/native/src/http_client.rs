use anyhow::anyhow;
use reqwest::Client;

pub struct HttpClient {
    base_url: String,
    client: Client,
}

impl std::ops::Deref for HttpClient {
    type Target = Client;
    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl HttpClient {
    pub fn new(url: &str) -> Self {
        Self {
            base_url: url.strip_suffix('/').unwrap_or(url).to_string(),
            client: Client::new(),
        }
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
        let url = format!("{}{}", self.base_url, path);

        let mut req = self.client.request(method.clone(), &url);
        req = headers.iter().fold(req, |req, (k, v)| req.header(*k, *v));

        if let Some(bytes) = body {
            req = req.body(bytes.to_vec());
        }

        let res = req.send().await?;
        let status = res.status();
        let bytes = res.bytes().await?;

        if !status.is_success() {
            let body = String::from_utf8_lossy(&bytes);
            return Err(anyhow!("request to {} failed ({}): {}", path, status, body));
        }

        Ok(bytes.to_vec())
    }
}
