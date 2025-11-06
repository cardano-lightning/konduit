use async_trait::async_trait;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_aux::prelude::deserialize_number_from_string;
use serde_json::json;
use std::time::Duration;

use crate::{
    QuoteRequest,
    bln::{
        BlnError, BlnInterface,
        interface::{PayRequest, PayResponse, QuoteResponse},
    },
};

#[derive(Debug, Clone, clap::Args)]
pub struct LndArgs {
    #[arg(long, env = "KONDUIT_BLN_URL")]
    pub bln_url: String,
    #[arg(long, env = "KONDUIT_BLN_TLS")]
    pub bln_tls: Option<Vec<u8>>,
    #[arg(long, env = "KONDUIT_BLN_MACAROON")]
    pub bln_macaroon: String,
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
        Self::new(
            value.bln_url.clone(),
            value.bln_tls.as_deref(),
            value.bln_macaroon.clone(),
        )
    }
}

impl WithLnd {
    pub fn new(
        base_url: String,
        tls_certificate: Option<&[u8]>,
        macaroon_hex: String,
    ) -> Result<Self, BlnError> {
        if base_url.is_empty() {
            return Err(BlnError::Initialization(
                "missing/invalid lightning base url".to_string(),
            ));
        }
        if macaroon_hex.is_empty() {
            return Err(BlnError::Initialization(
                "missing/invalid macaroon".to_string(),
            ));
        }

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
        let (dest, amt_msat) = match quote_request.clone() {
            QuoteRequest::Bolt11(invoice) => {
                (hex::encode(invoice.payee_compressed), invoice.amount_msat)
            }
        };
        let info_json = self.get(&format!("v1/getinfo")).await?;

        let info: GetInfo = serde_json::from_value(info_json)
            .map_err(|e| BlnError::Parse(format!("Failed to parse info: {}", e)))?;
        log::info!("INFO : {:?}", info);

        let route_json = self
            .get(&format!("v1/graph/routes/{}/0?amt_msat={}", dest, amt_msat))
            .await?;
        log::info!("{:?}", route_json);

        let routes: LndRoutes = serde_json::from_value(route_json)
            .map_err(|e| BlnError::Parse(format!("Failed to parse route: {}", e)))?;

        let route = routes.routes.get(0).ok_or_else(|| BlnError::ApiError {
            status: 404,
            message: "No route found".to_string(),
        })?;

        log::info!("{:?}", routes);

        let fee_msat = route.total_fees_msat;
        // FIXME:
        // The average block is ~10 minutes = 600seconds.
        // However, this is probablistic, and is subject to parameters that change every 2016 blocks.
        // if final ctlv is 80 and each hop is 40 this is a very long hold period.
        let estimated_timeout =
            Duration::from_secs((route.total_time_lock - info.block_height) * 10 * 60);

        match quote_request {
            QuoteRequest::Bolt11(invoice) => Ok(QuoteResponse {
                amount_msat: invoice.amount_msat,
                fee_msat,
                estimated_timeout,
            }),
        }
    }

    async fn pay(&self, req: PayRequest) -> Result<PayResponse, BlnError> {
        let request_body = PayRequestBody {
            dest: base64::engine::general_purpose::STANDARD.encode(req.recipient),
            payment_hash: base64::engine::general_purpose::STANDARD.encode(req.payment_hash),
            amt_msat: req.amount_msat.to_string(),
            // **Assumption**: Trait's `expiry` is the `final_cltv_delta`
            final_cltv_delta: req.expiry,
            // **Assumption**: Trait's `routing_fee` is the `fee_limit` in msat
            fee_limit: FeeLimit {
                fixed_msat: req.routing_fee.to_string(),
            },
            payment_addr: BASE64_STANDARD.encode(req.payment_secret),
        };

        let response_json = self.post("v1/channels/transactions", &request_body).await?;

        let pay_res: PayApiResponse = serde_json::from_value(response_json)
            .map_err(|e| BlnError::Parse(format!("Failed to parse pay response: {}", e)))?;

        if !pay_res.payment_error.is_empty() {
            return Err(BlnError::ApiError {
                status: 500,
                message: format!("Payment failed: {}", pay_res.payment_error),
            });
        }

        let secret = base64::engine::general_purpose::STANDARD.decode(&pay_res.payment_preimage)?;

        Ok(PayResponse {
            secret: secret.as_slice().try_into()?,
        })
    }
}

#[derive(Serialize)]
struct FeeLimit {
    fixed_msat: String,
}

#[derive(Serialize)]
struct PayRequestBody {
    dest: String,
    payment_hash: String,
    amt_msat: String,
    final_cltv_delta: u64,
    fee_limit: FeeLimit,
    payment_addr: String,
}

#[derive(Deserialize)]
struct PayReqResponse {
    destination: String,
    payment_hash: String,
    num_msat: String,
    cltv_expiry: String,
    payment_addr: String,
}

#[derive(Deserialize)]
struct PayApiResponse {
    payment_preimage: String,
    payment_error: String,
}

#[derive(Debug, Deserialize)]
struct GetInfo {
    block_height: u64,
}

#[derive(Deserialize, Debug)]
struct LndRoutes {
    routes: Vec<LndRoute>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct LndRoute {
    pub custom_channel_data: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub first_hop_amount_msat: u64,
    pub hops: Vec<Hop>,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub total_amt: u64,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub total_amt_msat: u64,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub total_fees: u64,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub total_fees_msat: u64,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub total_time_lock: u64,
}

/// Represents a single hop in a route.
#[derive(Serialize, Deserialize, Debug)]
pub struct Hop {
    //pub amp_record: Option<Value>,
    //pub mpp_record: Option<Value>,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub amt_to_forward: u64,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub amt_to_forward_msat: u64,
    pub blinding_point: String,
    pub encrypted_data: String,
    pub metadata: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub chan_capacity: u64,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub chan_id: u64,
    //pub custom_records: HashMap<String, Value>,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub expiry: u64,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub fee: u64,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub fee_msat: u64,
    #[serde(with = "hex")]
    pub pub_key: [u8; 33],
    pub tlv_payload: bool,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub total_amt_msat: u64,
}
