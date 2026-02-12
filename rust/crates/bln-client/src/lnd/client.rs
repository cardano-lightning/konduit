use async_trait::async_trait;
use futures::StreamExt;
use reqwest::{Method, RequestBuilder};
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use super::types::{get_info, graph_routes, payments, router_send, stream_wrapper};

use crate::{
    Api, Error, PayRequest, PayResponse, QuoteRequest, QuoteResponse, RevealRequest,
    RevealResponse, lnd::Config,
};

#[derive(Debug)]
pub struct Client {
    config: Config,
    client: reqwest::Client,
    lookup_table: Arc<Mutex<HashMap<[u8; 32], ([u8; 32], u64)>>>,
    last_cache_update: Arc<Mutex<u64>>,
}

impl TryFrom<Config> for Client {
    type Error = Error;

    fn try_from(value: Config) -> crate::Result<Self> {
        let mut client_builder = reqwest::Client::builder().timeout(Duration::from_secs(30));
        if let Some(cert_bytes) = value.tls_certificate.as_ref() {
            let cert = reqwest::Certificate::from_pem(cert_bytes)
                .map_err(|e| Error::Init(format!("Failed to parse PEM: {}", e)))?;
            client_builder = client_builder.add_root_certificate(cert);
        } else {
            client_builder = client_builder.danger_accept_invalid_certs(true);
        }

        let client = client_builder
            .build()
            .map_err(|e| Error::Init(format!("Failed to build client: {}", e)))?;

        Ok(Self {
            config: value,
            client,
            lookup_table: Arc::new(Mutex::new(HashMap::new())),
            last_cache_update: Arc::new(Mutex::new(0)),
        })
    }
}

impl Client {
    fn url(&self, path: &str) -> String {
        format!("{}/{}", self.config.base_url, path)
    }

    fn request(&self, method: Method, path: &str) -> RequestBuilder {
        self.client
            .request(method, self.url(path))
            .header("Grpc-Metadata-Macaroon", hex::encode(&self.config.macaroon))
    }

    fn get(&self, path: &str) -> RequestBuilder {
        self.request(Method::GET, path)
    }

    fn post(&self, path: &str) -> RequestBuilder {
        self.request(Method::POST, path)
    }

    async fn execute<T: DeserializeOwned>(&self, builder: RequestBuilder) -> crate::Result<T> {
        let response = builder.send().await?;
        if !response.status().is_success() {
            let status = response.status().into();
            let message = response.text().await?;
            return Err(Error::ApiError { status, message });
        }
        Ok(response.json().await?)
    }

    /// Stream handler that parses until a JSON object is complete.
    async fn execute_stream<T, F>(
        &self,
        builder: RequestBuilder,
        is_terminal: F,
    ) -> crate::Result<T>
    where
        T: DeserializeOwned,
        F: Fn(&T) -> bool,
    {
        let response = builder.send().await?;
        if !response.status().is_success() {
            let status = response.status().into();
            let message = response.text().await?;
            return Err(Error::ApiError { status, message });
        }

        let mut stream = response.bytes_stream();
        let mut buffer = Vec::new();
        let mut total_bytes_received = 0;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| Error::Init(e.to_string()))?;
            total_bytes_received += chunk.len();
            buffer.extend_from_slice(&chunk);

            loop {
                let mut it = serde_json::Deserializer::from_slice(&buffer)
                    .into_iter::<stream_wrapper::StreamWrapper<T>>();
                match it.next() {
                    Some(Ok(wrapper)) => {
                        let offset = it.byte_offset();
                        let terminal = is_terminal(&wrapper.result);
                        if terminal {
                            return Ok(wrapper.result);
                        }
                        buffer.drain(..offset);
                        while !buffer.is_empty() && buffer[0].is_ascii_whitespace() {
                            buffer.remove(0);
                        }
                        continue;
                    }
                    Some(Err(e)) if e.is_eof() => {
                        break;
                    }
                    Some(Err(e)) => {
                        let snippet_len = std::cmp::min(buffer.len(), 100);
                        let snippet = String::from_utf8_lossy(&buffer[..snippet_len]);
                        println!(
                            "DEBUG: JSON Parse Error: {}. Buffer start: {}...",
                            e, snippet
                        );

                        if let Some(pos) = buffer.iter().position(|&b| b == b'\n') {
                            println!(
                                "DEBUG: Found newline at pos {}. Skipping invalid line.",
                                pos
                            );
                            buffer.drain(..pos + 1);
                            continue;
                        }
                        return Err(Error::ApiError {
                            status: 500,
                            message: format!("Unrecoverable JSON parse error in stream: {}", e),
                        });
                    }
                    None => break,
                }
            }
        }

        Err(Error::ApiError {
            status: 500,
            message: format!(
                "Stream closed without reaching terminal state. Total bytes received: {}",
                total_bytes_received
            ),
        })
    }

    pub async fn v1_getinfo(&self) -> crate::Result<get_info::GetInfo> {
        self.execute(self.get("v1/getinfo")).await
    }

    pub async fn v1_graph_routes(
        &self,
        payee: [u8; 33],
        amount: u64,
    ) -> crate::Result<graph_routes::GraphRoutes> {
        let path = format!("v1/graph/routes/{}/{}", hex::encode(payee), amount);
        self.execute(self.get(&path)).await
    }

    pub async fn v1_payments(
        &self,
        query: &payments::Request,
    ) -> crate::Result<payments::Response> {
        self.execute(self.get("v1/payments").query(query)).await
    }

    pub async fn v2_router_send(
        &self,
        body: router_send::Request,
    ) -> crate::Result<router_send::Response> {
        let builder = self.post("v2/router/send").json(&body);
        self.execute_stream(builder, |res: &router_send::Response| {
            res.status == "SUCCEEDED" || res.status == "FAILED"
        })
        .await
    }

    async fn block_height(&self) -> crate::Result<u64> {
        self.v1_getinfo().await.map(|x| x.block_height as u64)
    }
}

#[async_trait]
impl Api for Client {
    async fn quote(&self, req: QuoteRequest) -> crate::Result<QuoteResponse> {
        let routes = self.v1_graph_routes(req.payee, req.amount_msat).await?;
        let route = routes.routes.first().ok_or(Error::ApiError {
            status: 404,
            message: "No route".into(),
        })?;

        let blocks = route
            .total_time_lock
            .checked_sub(self.block_height().await?)
            .ok_or(Error::Time)?;
        let relative_timeout = self
            .config
            .block_time
            .checked_mul(blocks as u32)
            .ok_or(Error::Time)?;

        Ok(QuoteResponse {
            relative_timeout,
            fee_msat: route.total_fees_msat,
        })
    }

    async fn pay(&self, req: PayRequest) -> crate::Result<PayResponse> {
        let blocks = req.relative_timeout.as_secs() / self.config.block_time.as_secs();
        let body = router_send::Request {
            cltv_limit: Some(std::cmp::max(blocks, self.config.min_cltv)),
            fee_limit_msat: Some(req.fee_limit),
            payment_request: Some(req.invoice.into()),
            ..Default::default()
        };

        let res = self.v2_router_send(body).await?;
        if res.status == "FAILED" {
            Err(Error::ApiError {
                status: 500,
                message: format!("LND Payment Failed: {}", res.payment_error),
            })
        } else if res.payment_preimage.is_empty() {
            Err(Error::ApiError {
                status: 503,
                message: "Payment succeeded but no preimage returned".into(),
            })
        } else {
            Ok(PayResponse {
                secret: res.payment_preimage.as_slice().try_into().ok(),
            })
        }
    }

    async fn reveal(&self, req: RevealRequest) -> crate::Result<RevealResponse> {
        {
            let cache = self.lookup_table.lock().await;
            if let Some(secret) = cache.get(&req.lock) {
                return Ok(RevealResponse {
                    secret: Some(secret.0),
                });
            }
        }

        let last_update = *self.last_cache_update.lock().await;
        let res = self
            .v1_payments(&payments::Request {
                index_offset: Some(last_update),
                include_incomplete: false,
                ..Default::default()
            })
            .await?;

        let mut cache = self.lookup_table.lock().await;
        let mut last_idx = self.last_cache_update.lock().await;

        *last_idx = res.last_index_offset;
        for p in res.payments {
            if let Some(preimage) = p.payment_preimage {
                cache.insert(p.payment_hash, (preimage, p.payment_index));
            }
        }

        if cache.len() > self.config.max_cache_size {
            let threshold = res
                .last_index_offset
                .saturating_sub((self.config.max_cache_size / 2) as u64);
            cache.retain(|_, v| v.1 >= threshold);
        }

        Ok(RevealResponse {
            secret: cache.get(&req.lock).map(|s| s.0),
        })
    }
}
