use async_trait::async_trait;
use reqwest::RequestBuilder;
use serde::{Serialize, de::DeserializeOwned};
use std::time::Duration;

use super::types::{GetInfo, Route, RouterSendRequest, Routes, SendPaymentResponse};

use crate::{Api, Error, PayRequest, PayResponse, QuoteRequest, QuoteResponse, lnd::Config};

#[derive(Debug)]
pub struct Client {
    config: Config,
    client: reqwest::Client,
}

impl TryFrom<Config> for Client {
    type Error = Error;

    fn try_from(value: Config) -> Result<Self, Self::Error> {
        let mut client_builder = reqwest::Client::builder().timeout(Duration::from_secs(60));
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
        })
    }
}

impl Client {
    async fn execute<T: DeserializeOwned>(&self, builder: RequestBuilder) -> crate::Result<T> {
        let response = builder
            .header(
                "Grpc-Metadata-Macaroon",
                &hex::encode(&self.config.macaroon),
            )
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json::<T>().await?)
        } else {
            let status = response.status().into();
            let message = response.text().await?;
            Err(Error::ApiError { status, message })
        }
    }

    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> crate::Result<T> {
        let url = format!("{}/{}", &self.config.base_url, path);
        self.execute(self.client.get(&url)).await
    }

    pub async fn post<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: B,
    ) -> crate::Result<T> {
        let url = format!("{}/{}", &self.config.base_url, path);
        self.execute(self.client.post(&url).json(&body)).await
    }

    pub async fn v1_getinfo(&self) -> crate::Result<GetInfo> {
        self.get("v1/getinfo").await
    }

    pub async fn v1_graph_routes(
        &self,
        payee: [u8; 33],
        amount_msat: u64,
    ) -> crate::Result<Routes> {
        self.get(&format!(
            "v1/graph/routes/{}/{}",
            hex::encode(payee),
            amount_msat
        ))
        .await?
    }

    pub async fn v2_router_send(
        &self,
        body: RouterSendRequest,
    ) -> crate::Result<SendPaymentResponse> {
        self.post("v2/router/send", &body).await
    }

    async fn block_height(&self) -> crate::Result<u64> {
        self.v1_getinfo().await.map(|x| x.block_height as u64)
    }

    async fn find_route(&self, payee: [u8; 33], amount_msat: u64) -> crate::Result<Route> {
        let routes = self.v1_graph_routes(payee, amount_msat).await?;
        // FIXME :: Take first route. We have no knowledge of what to do when there are multiple
        let route = routes.routes.first().ok_or_else(|| Error::ApiError {
            status: 404,
            message: "No route found".to_string(),
        })?;
        Ok(route.clone())
    }
}

#[async_trait]
impl Api for Client {
    async fn quote(&self, quote_request: QuoteRequest) -> crate::Result<QuoteResponse> {
        let route = self
            .find_route(quote_request.payee, quote_request.amount_msat)
            .await?;
        let fee_msat = route.total_fees_msat;

        let Some(blocks) = route
            .total_time_lock
            .checked_sub(self.block_height().await?)
        else {
            return Err(Error::Time);
        };
        // FIXME :: See [NOTE ON TIME]
        let Some(relative_timeout) = self.config.block_time.checked_mul(blocks as u32) else {
            return Err(Error::Time);
        };
        Ok(QuoteResponse {
            relative_timeout,
            fee_msat,
        })
    }

    async fn pay(&self, req: PayRequest) -> Result<PayResponse, Error> {
        let blocks = req.relative_timeout.as_secs() / self.config.block_time.as_secs();
        let cltv_limit = std::cmp::max(blocks, self.config.min_cltv);

        let invoice_str: String = req.invoice.into();
        let body = RouterSendRequest {
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

        let pay_res = self.v2_router_send(body).await?;
        if !pay_res.payment_error.is_empty() {
            return Err(Error::ApiError {
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
