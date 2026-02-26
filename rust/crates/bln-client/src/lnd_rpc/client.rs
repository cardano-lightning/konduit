use crate::{
    Api, Error,
    lnd_rpc::{
        config::Config,
        interceptor::Interceptor,
        proto::{lnrpc, routerrpc},
    },
    types::{PayRequest, PayResponse, QuoteRequest, QuoteResponse, RevealRequest, RevealResponse},
};
use async_trait::async_trait;
use base64::Engine;
use lnrpc::lightning_client::LightningClient;
use routerrpc::router_client::RouterClient;
use std::time::Duration;
use tonic::{
    Request,
    transport::{Certificate, Channel, ClientTlsConfig},
};

/// The gRPC implementation of the LND client.
pub struct Client {
    config: Config,
    lightning:
        LightningClient<tonic::service::interceptor::InterceptedService<Channel, Interceptor>>,
    router: RouterClient<tonic::service::interceptor::InterceptedService<Channel, Interceptor>>,
}

impl Client {
    /// Create a new RPC client from configuration.
    pub async fn new(config: Config) -> crate::Result<Self> {
        // 1. Setup TLS if provided
        let tls = if let Some(tls) = &config.tls {
            let ca = Certificate::from_pem(tls);
            Some(ClientTlsConfig::new().ca_certificate(ca))
        } else {
            None
        };

        // 2. Create the channel
        let mut endpoint = Channel::from_shared(config.base_url.clone())
            .map_err(|e| Error::Init(format!("Invalid base URL: {}", e)))?
            .connect_timeout(Duration::from_secs(10));

        if let Some(tls) = tls {
            endpoint = endpoint
                .tls_config(tls)
                .map_err(|e| Error::Init(format!("TLS error: {}", e)))?;
        }

        let channel = endpoint
            .connect()
            .await
            .map_err(|e| Error::Init(format!("Connection failed: {}", e)))?;

        // 3. Setup Interceptor with Macaroon
        let interceptor = Interceptor::new(config.macaroon.clone());

        // 4. Initialize clients
        let lightning = LightningClient::with_interceptor(channel.clone(), interceptor.clone());
        let router = RouterClient::with_interceptor(channel, interceptor);

        Ok(Self {
            config,
            lightning,
            router,
        })
    }

    async fn get_block_height(&self) -> crate::Result<u32> {
        let resp = self
            .lightning
            .clone()
            .get_info(Request::new(lnrpc::GetInfoRequest {}))
            .await
            .map_err(|e| Error::ApiError {
                status: 500,
                message: format!("RPC GetInfo failed: {}", e.message()),
            })?;
        Ok(resp.into_inner().block_height)
    }
}

#[async_trait]
impl Api for Client {
    async fn quote(&self, req: QuoteRequest) -> crate::Result<QuoteResponse> {
        // Query LND for routes
        let lnd_req = lnrpc::QueryRoutesRequest {
            pub_key: hex::encode(req.payee),
            amt_msat: req.amount_msat as i64,
            use_mission_control: true,
            ..Default::default()
        };

        let response = self
            .lightning
            .clone()
            .query_routes(Request::new(lnd_req))
            .await
            .map_err(|e| Error::ApiError {
                status: 500,
                message: format!("QueryRoutes failed: {}", e.message()),
            })?;

        let routes = response.into_inner().routes;
        let route = routes.first().ok_or(Error::ApiError {
            status: 404,
            message: "No route found".to_string(),
        })?;

        // Calculate relative timeout
        let current_height = self.get_block_height().await?;
        let blocks_diff = route
            .total_time_lock
            .checked_sub(current_height)
            .ok_or(Error::Time)?;

        let relative_timeout = self
            .config
            .block_time
            .checked_mul(blocks_diff)
            .ok_or(Error::Time)?;

        Ok(QuoteResponse {
            fee_msat: route.total_fees_msat as u64,
            relative_timeout,
        })
    }

    async fn pay(&self, req: PayRequest) -> crate::Result<PayResponse> {
        let invoice_str: String = req.invoice.into();

        // Estimate CLTV limit from duration
        let cltv_limit = (req.relative_timeout.as_secs() / self.config.block_time.as_secs()) as u32;

        let lnd_req = routerrpc::SendPaymentRequest {
            payment_request: invoice_str,
            fee_limit_msat: req.fee_limit as i64,
            cltv_limit: std::cmp::max(cltv_limit, self.config.min_cltv as u32) as i32,
            timeout_seconds: 60,
            ..Default::default()
        };

        let mut stream = self
            .router
            .clone()
            .send_payment_v2(Request::new(lnd_req))
            .await
            .map_err(|e| Error::ApiError {
                status: 500,
                message: format!("SendPayment failed: {}", e.message()),
            })?
            .into_inner();

        // Listen for terminal state in the stream
        while let Some(update) = stream.message().await.map_err(|e| Error::ApiError {
            status: 500,
            message: format!("Payment stream error: {}", e.message()),
        })? {
            match update.status() {
                lnrpc::payment::PaymentStatus::Succeeded => {
                    let secret_str = update.payment_preimage;
                    let secret = if secret_str.is_empty() {
                        None
                    } else {
                        Some(try_to_arr32(&secret_str).ok_or(Error::ApiError {
                            status: 500,
                            message: format!("Cannot handle payment preimage :: {:?}", secret_str),
                        })?)
                    };
                    return Ok(PayResponse { secret });
                }
                lnrpc::payment::PaymentStatus::Failed => {
                    return Err(Error::ApiError {
                        status: 500,
                        message: format!("Payment failed: {:?}", update.failure_reason()),
                    });
                }
                _ => continue, // Still in flight
            }
        }

        Err(Error::ApiError {
            status: 500,
            message: "Stream closed without success or failure".to_string(),
        })
    }

    async fn reveal(&self, req: RevealRequest) -> crate::Result<RevealResponse> {
        // Query ListPayments to find the preimage for the given hash (lock)
        let lnd_req = lnrpc::ListPaymentsRequest {
            include_incomplete: false,
            ..Default::default()
        };

        let response = self
            .lightning
            .clone()
            .list_payments(Request::new(lnd_req))
            .await
            .map_err(|e| Error::ApiError {
                status: 500,
                message: format!("ListPayments failed: {}", e.message()),
            })?;

        let payments = response.into_inner().payments;
        let lock_hex = hex::encode(req.lock);

        let found = payments.into_iter().find(|p| p.payment_hash == lock_hex);

        match found {
            Some(p) => {
                let secret = hex::decode(p.payment_preimage)
                    .ok()
                    .and_then(|v| v.try_into().ok());
                Ok(RevealResponse { secret })
            }
            None => Ok(RevealResponse { secret: None }),
        }
    }
}

fn try_to_arr32(s: &str) -> Option<[u8; 32]> {
    let vec = if s.len() == 64 {
        hex::decode(s).ok()?
    } else {
        base64::engine::general_purpose::STANDARD.decode(s).ok()?
    };
    <[u8; 32]>::try_from(vec).ok()
}
