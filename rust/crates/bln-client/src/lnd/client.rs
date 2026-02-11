use async_trait::async_trait;
use reqwest::RequestBuilder;
use serde::{Serialize, de::DeserializeOwned};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::types::{get_info, graph_routes, payments, send_payment};

use crate::{
    Api, Error, PayRequest, PayResponse, QuoteRequest, QuoteResponse, RevealRequest,
    RevealResponse, lnd::Config,
};

#[derive(Debug)]
pub struct Client {
    config: Config,
    client: reqwest::Client,
}

impl TryFrom<Config> for Client {
    type Error = Error;

    fn try_from(value: Config) -> crate::Result<Self> {
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
            let body_text = response.text().await?;

            match serde_json::from_str::<T>(&body_text) {
                Ok(data) => Ok(data),
                Err(e) => {
                    eprintln!("{:?}", e);
                    panic!("{:?}", body_text);
                    let context = get_error_context(&body_text, e.line(), e.column());
                }
            }
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

    pub async fn get_query<T: DeserializeOwned, U: Serialize + Sized>(
        &self,
        path: &str,
        query: &U,
    ) -> crate::Result<T> {
        let url = format!("{}/{}", &self.config.base_url, path);
        self.execute(self.client.get(&url).query(query)).await
    }

    pub async fn post<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: B,
    ) -> crate::Result<T> {
        let url = format!("{}/{}", &self.config.base_url, path);
        self.execute(self.client.post(&url).json(&body)).await
    }

    pub async fn v1_getinfo(&self) -> crate::Result<get_info::GetInfo> {
        self.get("v1/getinfo").await
    }

    pub async fn v1_graph_routes(
        &self,
        payee: [u8; 33],
        amount_msat: u64,
    ) -> crate::Result<graph_routes::GraphRoutes> {
        self.get(&format!(
            "v1/graph/routes/{}/{}",
            hex::encode(payee),
            amount_msat
        ))
        .await
    }

    pub async fn v1_payments(
        &self,
        query: &payments::PaymentsRequest,
    ) -> crate::Result<payments::PaymentsResponse> {
        self.get_query("v1/payments", query).await
    }

    pub async fn v2_router_send(
        &self,
        body: send_payment::RouterSendRequest,
    ) -> crate::Result<send_payment::SendPaymentResponse> {
        self.post("v2/router/send", &body).await
    }

    async fn block_height(&self) -> crate::Result<u64> {
        self.v1_getinfo().await.map(|x| x.block_height as u64)
    }

    async fn find_route(
        &self,
        payee: [u8; 33],
        amount_msat: u64,
    ) -> crate::Result<graph_routes::Route> {
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

    async fn pay(&self, req: PayRequest) -> crate::Result<PayResponse> {
        let blocks = req.relative_timeout.as_secs() / self.config.block_time.as_secs();
        let cltv_limit = std::cmp::max(blocks, self.config.min_cltv);

        let invoice_str: String = req.invoice.into();
        let body = send_payment::RouterSendRequest {
            // amt_msat: Some(req.amount_msat),
            cltv_limit: Some(cltv_limit),
            fee_limit_msat: Some(req.fee_limit),
            // dest: Some(req.payee),
            // payment_hash: Some(req.payment_hash),
            // payment_addr: Some(req.payment_secret),
            payment_request: Some(invoice_str),
            // final_cltv_delta: Some(req.final_cltv_delta),
            ..send_payment::RouterSendRequest::default()
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
            secret: secret.as_slice().try_into().ok(),
        })
    }

    // FIXME:: This is awful.
    async fn reveal(&self, req: RevealRequest) -> crate::Result<RevealResponse> {
        let two_weeks_secs = 14 * 24 * 60 * 60;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let start_time = now.saturating_sub(two_weeks_secs);
        let query = payments::PaymentsRequest {
            include_incomplete: false,
            // creation_date_start: Some(start_time),
            ..Default::default()
        };

        let res = self.v1_payments(&query).await?;

        if let Some(target) = res
            .payments
            .into_iter()
            .find(|p| p.payment_hash == req.lock)
        {
            let secret = target.payment_preimage;
            Ok(RevealResponse { secret })
        } else {
            Err(Error::InvalidData(format!(
                "No hash {}",
                hex::encode(&req.lock.to_vec())
            )))
        }
    }
}

fn get_error_context(text: &str, line: usize, col: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    if line == 0 || line > lines.len() {
        return format!("(Line {} out of bounds or empty)", line);
    }

    let target_line = lines[line - 1];
    // Calculate a window around the column
    let start = col.saturating_sub(40);
    let end = (col + 40).min(target_line.len());
    let snippet = &target_line[start..end];

    format!(
        "\nLine {}, Col {}: ... {} ...\n{: >width$}^",
        line,
        col,
        snippet,
        "",
        width = (col - start) + 11 // Offset for "Line X, Col Y: ... "
    )
}
