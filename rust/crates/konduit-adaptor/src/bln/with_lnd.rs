use async_trait::async_trait;
use reqwest::Client;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

mod models;

use crate::{
    BlnInterface, QuoteRequest,
    bln::{
        BlnError,
        interface::{PayRequest, PayResponse, QuoteResponse},
        with_lnd::models::{
            FeeLimit, GetInfo, Route, Routes, SendPaymentRequest, SendPaymentResponse,
        },
    },
};

#[derive(Debug, Clone, clap::Args)]
pub struct LndArgs {
    #[arg(long, env = crate::env::BLN_URL)]
    pub bln_url: String,
    #[arg(long, env = crate::env::BLN_TLS)]
    pub bln_tls: Option<Vec<u8>>,
}

#[derive(Debug)]
pub struct WithLnd {
    base_url: String,
    macaroon_hex: String,
    client: Client,
}

impl TryFrom<&LndArgs> for WithLnd {
    type Error = BlnError;

    fn try_from(value: &LndArgs) -> Result<Self, Self::Error> {
        Self::new(value.bln_url.clone(), value.bln_tls.as_deref(), None)
    }
}

impl WithLnd {
    pub fn new(
        base_url: String,
        tls_certificate: Option<&[u8]>,
        macaroon_hex: Option<String>,
    ) -> Result<Self, BlnError> {
        if base_url.is_empty() {
            return Err(BlnError::Initialization(
                "missing/invalid lightning base url".to_string(),
            ));
        }
        let macaroon_hex = if let Some(hex) = macaroon_hex {
            hex
        } else {
            std::env::var(crate::env::BLN_MACAROON)
                .map_err(|_| BlnError::Initialization("missing/invalid macaroon".to_string()))?
        };

        let mut client_builder = Client::builder().timeout(Duration::from_secs(60));

        if let Some(cert_bytes) = tls_certificate {
            let cert = reqwest::Certificate::from_pem(cert_bytes)
                .map_err(|e| BlnError::Initialization(format!("Failed to parse PEM: {}", e)))?;
            client_builder = client_builder.add_root_certificate(cert);
        } else {
            client_builder = client_builder.danger_accept_invalid_certs(true);
        }

        let client = client_builder
            .build()
            .map_err(|e| BlnError::Initialization(format!("Failed to build client: {}", e)))?;

        Ok(Self {
            base_url,
            macaroon_hex,
            client,
        })
    }

    async fn find_route(&self, payee: [u8; 33], amount_msat: u64) -> Result<Route, BlnError> {
        let route_json = self
            .get(&format!(
                "v1/graph/routes/{}/0?amt_msat={}",
                hex::encode(payee),
                amount_msat
            ))
            .await?;

        let routes: Routes = serde_json::from_value(route_json)
            .map_err(|e| BlnError::Parse(format!("Failed to parse route: {}", e)))?;

        // FIXME :: Take first route. We have no knowledge of what to do when there are multiple
        let route = routes.routes.first().ok_or_else(|| BlnError::ApiError {
            status: 404,
            message: "No route found".to_string(),
        })?;
        Ok(route.clone())
    }

    async fn block_height(&self) -> Result<u64, BlnError> {
        let info_json = self.get("v1/getinfo").await?;

        let info: GetInfo = serde_json::from_value(info_json)
            .map_err(|e| BlnError::Parse(format!("Failed to parse info: {}", e)))?;
        Ok(info.block_height)
    }

    /// Helper to handle API errors
    async fn handle_response(
        &self,
        response: reqwest::Response,
    ) -> Result<serde_json::Value, BlnError> {
        if response.status().is_success() {
            return Ok(response.json().await?);
        }
        let status = response.status().as_u16();
        let message = response
            .text()
            .await
            .unwrap_or("No message given".to_string());
        Err(BlnError::ApiError { status, message })
    }

    async fn get(&self, path: &str) -> Result<serde_json::Value, BlnError> {
        let url = format!("{}/{}", self.base_url, path);
        let response = self
            .client
            .get(&url)
            .header("Grpc-Metadata-Macaroon", &self.macaroon_hex)
            .send()
            .await?;
        if response.status().is_success() {
            self.handle_response(response).await
        } else {
            panic!("{:?}", response.text().await?);
        }
    }

    async fn post(
        &self,
        path: &str,
        body: impl serde::Serialize,
    ) -> Result<serde_json::Value, BlnError> {
        let url = format!("{}/{}", self.base_url, path);
        let response = self
            .client
            .post(&url)
            .header("Grpc-Metadata-Macaroon", &self.macaroon_hex)
            .json(&body)
            .send()
            .await?;
        self.handle_response(response).await
    }
}

#[async_trait]
impl BlnInterface for WithLnd {
    async fn quote(&self, quote_request: QuoteRequest) -> Result<QuoteResponse, BlnError> {
        let route = self
            .find_route(quote_request.payee, quote_request.amount_msat)
            .await?;
        let fee_msat = route.total_fees_msat;

        // FIXME :: [NOTE ON TIME]
        // The average block is ~10 minutes = 600seconds.
        // However, this is probablistic, and is subject to parameters that change every 2016 blocks.
        // if final ctlv is 80 and each hop is 40 this is a very long hold period.
        let estimated_timeout =
            Duration::from_secs((route.total_time_lock - self.block_height().await?) * 10 * 60);

        Ok(QuoteResponse {
            fee_msat,
            estimated_timeout,
        })
    }

    async fn pay(&self, req: PayRequest) -> Result<PayResponse, BlnError> {
        let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) else {
            return Err(BlnError::Time);
        };
        // FIXME :: See [NOTE ON TIME]
        let estimate_relative_timeout_blocks = (req.timeout.as_secs() - now.as_secs()) / (10 * 60);
        let cltv_limit = self.block_height().await? + estimate_relative_timeout_blocks;
        let fee_limit = FeeLimit {
            fixed_msat: Some(req.routing_fee),
            ..FeeLimit::default()
        };

        let request_body = SendPaymentRequest {
            amt_msat: Some(req.amount_msat),
            cltv_limit: Some(cltv_limit),
            fee_limit: Some(fee_limit),
            dest: Some(req.payee),
            payment_hash: Some(req.payment_hash),
            payment_addr: Some(req.payment_secret),
            final_cltv_delta: Some(req.final_cltv_delta),
            ..SendPaymentRequest::default()
        };

        let response_json = self.post("v1/channels/transactions", &request_body).await?;

        let pay_res: SendPaymentResponse = serde_json::from_value(response_json)
            .map_err(|e| BlnError::Parse(format!("Failed to parse pay response: {}", e)))?;

        if !pay_res.payment_error.is_empty() {
            return Err(BlnError::ApiError {
                status: 500,
                message: format!("Payment failed: {}", pay_res.payment_error),
            });
        }

        let secret = &pay_res.payment_preimage;

        Ok(PayResponse {
            secret: secret.as_slice().try_into()?,
        })
    }
}
