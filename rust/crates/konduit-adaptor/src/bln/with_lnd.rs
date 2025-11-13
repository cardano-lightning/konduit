use async_trait::async_trait;
use reqwest::Client;
use std::time::Duration;

mod models;

use crate::{
    BlnInterface, QuoteRequest,
    bln::{
        BlnError,
        interface::{PayRequest, PayResponse, QuoteResponse},
        with_lnd::models::{GetInfo, Route, RouterSendRequest, Routes, SendPaymentResponse},
    },
};

/// FIXME :: [NOTE ON TIME]
/// The average block is ~10 minutes = 600seconds.
/// However, this is probablistic, and is subject to parameters that change every 2016 blocks.
/// if final ctlv is 80 and each hop is 40 this is a very long hold period.
/// This is an estimate
const BITCOIN_BLOCK_TIME: std::time::Duration = Duration::from_secs(600);
const LND_MIN_CLTV_LIMIT: u64 = 84;

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
    block_time: Duration,
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
            block_time: BITCOIN_BLOCK_TIME,
        })
    }

    async fn find_route(&self, payee: [u8; 33], amount_msat: u64) -> Result<Route, BlnError> {
        let route_json = self
            .get(&format!(
                "v1/graph/routes/{}/{}",
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

        let Some(blocks) = route
            .total_time_lock
            .checked_sub(self.block_height().await?)
        else {
            return Err(BlnError::Time);
        };
        // FIXME :: See [NOTE ON TIME]
        let Some(relative_timeout) = self.block_time.checked_mul(blocks as u32) else {
            return Err(BlnError::Time);
        };
        Ok(QuoteResponse {
            fee_msat,
            relative_timeout,
        })
    }

    async fn pay(&self, req: PayRequest) -> Result<PayResponse, BlnError> {
        let blocks = req.relative_timeout.as_secs() / self.block_time.as_secs();
        log::info!(
            "Calculated timeout: {} seconds -> {} blocks",
            req.relative_timeout.as_secs(),
            blocks
        );
        let cltv_limit = std::cmp::max(blocks, LND_MIN_CLTV_LIMIT);

        let invoice_str: String = req.invoice.into();
        let request_body = RouterSendRequest {
            // amt_msat: Some(req.amount_msat),
            cltv_limit: Some(cltv_limit),
            fee_limit_msat: Some(req.fee_limit),
            // dest: Some(req.payee),
            // payment_hash: Some(req.payment_hash),
            // payment_addr: Some(req.payment_secret),
            payment_request: Some(invoice_str),
            // final_cltv_delta: Some(req.final_cltv_delta),
            ..RouterSendRequest::default()
        };
        log::info!("request_body: {:?}", serde_json::to_string(&request_body));
        let response_json = self.post("v2/router/send", &request_body).await;
        log::info!("response_json: {:?}", response_json);
        let response_json = response_json?;

        let pay_res: SendPaymentResponse =
            serde_json::from_value(response_json.clone()).map_err(|e| {
                BlnError::Parse(format!(
                    "Failed to parse pay response: {} {}",
                    e, response_json
                ))
            })?;

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
