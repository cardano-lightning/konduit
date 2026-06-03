use anyhow::Context;

pub trait Transport: http_client::HttpClient<Error = anyhow::Error> {}
impl<H: http_client::HttpClient<Error = anyhow::Error>> Transport for H {}

/// An http client with more opinions
pub struct HttpClient<H: Transport> {
    http: H,
}

impl<H: Transport> HttpClient<H> {
    pub fn new(http: H) -> Self {
        Self { http }
    }

    pub fn base_url(&self) -> &str {
        self.http.base_url()
    }

    pub fn to_json<T: serde::Serialize>(value: &T) -> Vec<u8> {
        serde_json::to_vec(value)
            .unwrap_or_else(|e| unreachable!("failed to serialize to vector? {e}"))
    }

    pub async fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> anyhow::Result<T> {
        self.get_with_headers(path, &[]).await
    }

    pub async fn get_with_headers<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        headers: &[(&str, &str)],
    ) -> anyhow::Result<T> {
        let bytes = self.http.get(path, headers).await?;
        serde_json::from_slice(&bytes).with_context(|| {
            format!(
                "failed to parse response for type {}",
                std::any::type_name::<T>(),
            )
        })
    }

    pub async fn post<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: impl AsRef<[u8]>,
    ) -> anyhow::Result<T> {
        self.post_with_headers(path, &[], body).await
    }

    pub async fn post_with_headers<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        headers: &[(&str, &str)],
        body: impl AsRef<[u8]>,
    ) -> anyhow::Result<T> {
        let bytes = self.http.post(path, headers, body.as_ref()).await?;
        serde_json::from_slice(&bytes).with_context(|| {
            format!(
                "failed to parse response for type {}",
                std::any::type_name::<T>(),
            )
        })
    }
}
