use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub operation_id: String,
    pub expected_transaction_id: String,
    pub transaction: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Response {
    pub operation_id: String,
    pub expected_transaction_id: String,
    pub transaction_id: Option<String>,
    pub status: String,
    pub depth: Option<u64>,
}

#[async_trait]
pub trait Api: Send + Sync {
    async fn submit(&self, request: &Request) -> anyhow::Result<Response>;
    async fn lookup(&self, operation_id: &str) -> anyhow::Result<Response>;
}

pub struct Client {
    http: reqwest::Client,
    base: String,
    token: String,
}

impl Client {
    pub fn new(base: String, token: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            base: base.trim_end_matches('/').to_owned(),
            token,
        }
    }
}

#[async_trait]
impl Api for Client {
    async fn submit(&self, request: &Request) -> anyhow::Result<Response> {
        Ok(self
            .http
            .post(format!("{}/internal/channel-operations", self.base))
            .bearer_auth(&self.token)
            .json(request)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }

    async fn lookup(&self, operation_id: &str) -> anyhow::Result<Response> {
        Ok(self
            .http
            .get(format!(
                "{}/internal/channel-operations/{operation_id}",
                self.base
            ))
            .bearer_auth(&self.token)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }
}
