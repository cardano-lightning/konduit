use async_trait::async_trait;
use futures::StreamExt;
use reqwest::{Method, RequestBuilder};
use serde::{Serialize, de::DeserializeOwned};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use super::types::{get_info, graph_routes, payments, send_payment};

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
        // High timeout for pathfinding and multi-hop payments
        let mut client_builder = reqwest::Client::builder().timeout(Duration::from_secs(360));
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
        format!(
            "{}/{}",
            self.config.base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }

    fn request(&self, method: Method, path: &str) -> RequestBuilder {
        self.client
            .request(method, self.url(path))
            .header("Grpc-Metadata-Macaroon", hex::encode(&self.config.macaroon))
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

    /// Robust stream handler that handles concatenated JSON objects without newlines
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

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| Error::Init(e.to_string()))?;
            buffer.extend_from_slice(&chunk);

            // Attempt to parse objects from the buffer.
            // LND REST can send items separated by newlines OR just back-to-back JSON.
            // We use a StreamDeserializer approach.
            let mut cursor = std::io::Cursor::new(&buffer);
            let mut last_finish = 0;

            let mut stream_de = serde_json::Deserializer::from_reader(&mut cursor).into_iter::<T>();

            while let Some(Ok(data)) = stream_de.next() {
                last_finish = stream_de.byte_offset();
                if is_terminal(&data) {
                    return Ok(data);
                }
            }

            // Remove processed bytes from the buffer
            if last_finish > 0 {
                buffer.drain(..last_finish);
            }
        }

        Err(Error::ApiError {
            status: 500,
            message: "Stream connection closed without reaching terminal (SUCCEEDED/FAILED) state"
                .into(),
        })
    }

    pub async fn v1_getinfo(&self) -> crate::Result<get_info::GetInfo> {
        self.execute(self.request(Method::GET, "v1/getinfo")).await
    }

    pub async fn v1_graph_routes(
        &self,
        payee: [u8; 33],
        amount: u64,
    ) -> crate::Result<graph_routes::GraphRoutes> {
        let path = format!("v1/graph/routes/{}/{}", hex::encode(payee), amount);
        self.execute(self.request(Method::GET, &path)).await
    }

    pub async fn v1_payments(
        &self,
        query: &payments::PaymentsRequest,
    ) -> crate::Result<payments::PaymentsResponse> {
        self.execute(self.request(Method::GET, "v1/payments").query(query))
            .await
    }

    pub async fn v2_router_send(
        &self,
        body: send_payment::RouterSendRequest,
    ) -> crate::Result<send_payment::SendPaymentResponse> {
        let builder = self.request(Method::POST, "v2/router/send").json(&body);
        // Ensure we check for status strings correctly
        self.execute_stream(builder, |res: &send_payment::SendPaymentResponse| {
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
        let body = send_payment::RouterSendRequest {
            cltv_limit: Some(std::cmp::max(blocks, self.config.min_cltv)),
            fee_limit_msat: Some(req.fee_limit),
            payment_request: Some(req.invoice.into()),
            ..Default::default()
        };

        let res = self.v2_router_send(body).await?;
        if res.status == "FAILED" {
            return Err(Error::ApiError {
                status: 500,
                message: format!("LND Payment Failed: {}", res.payment_error),
            });
        }

        if res.payment_preimage.is_empty() {
            return Err(Error::ApiError {
                status: 500,
                message: "Payment succeeded but no preimage returned".into(),
            });
        }

        Ok(PayResponse {
            secret: res.payment_preimage.as_slice().try_into().ok(),
        })
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
            .v1_payments(&payments::PaymentsRequest {
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
