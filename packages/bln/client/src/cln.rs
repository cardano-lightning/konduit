use crate::{
    Api, CreateInvoiceRequest, CreatedInvoice, Error, Health,
    api::validate_commit_quote,
    types::{PayRequest, PayResponse, QuoteRequest, QuoteResponse, RevealRequest, RevealResponse},
};
use async_trait::async_trait;
use serde::Deserialize;
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

static INVOICE_LABEL_SEQUENCE: AtomicU64 = AtomicU64::new(0);
const CLN_SHADOW_ROUTING_MARGIN_BLOCKS: u32 = 135;

/// Core Lightning REST client for health checks and invoice creation.
#[derive(Debug, Clone)]
pub struct Client {
    base_url: String,
    rune: String,
    block_time: Duration,
    http: reqwest::Client,
}

impl Client {
    pub fn new(
        base_url: impl Into<String>,
        rune: impl Into<String>,
        block_time: Duration,
    ) -> crate::Result<Self> {
        if block_time.is_zero() {
            return Err(Error::InvalidData("block time must be positive".into()));
        }
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|error| Error::Init(error.to_string()))?;
        Ok(Self {
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            rune: rune.into(),
            block_time,
            http,
        })
    }

    pub async fn health(&self) -> crate::Result<Health> {
        let response = self
            .http
            .post(format!("{}/v1/getinfo", self.base_url))
            .header("Rune", &self.rune)
            .json(&serde_json::json!({}))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await?;
            return Err(Error::ApiError { status, message });
        }

        let info: GetInfo = response
            .json()
            .await
            .map_err(|error| Error::Parse(error.to_string()))?;
        Ok(info.into())
    }

    pub async fn create_invoice(
        &self,
        request: CreateInvoiceRequest,
    ) -> crate::Result<CreatedInvoice> {
        if request.amount_msat == 0 {
            return Err(Error::InvalidData(
                "invoice amount must be greater than zero".into(),
            ));
        }

        #[derive(serde::Serialize)]
        struct AddInvoiceRequest {
            amount_msat: String,
            label: String,
            description: String,
        }

        #[derive(Deserialize)]
        struct AddInvoiceResponse {
            bolt11: String,
        }

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| Error::Init(error.to_string()))?
            .as_nanos();
        let sequence = INVOICE_LABEL_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let response: AddInvoiceResponse = self
            .execute(
                self.http
                    .post(format!("{}/v1/invoice", self.base_url))
                    .header("Rune", &self.rune)
                    .json(&AddInvoiceRequest {
                        amount_msat: format!("{}msat", request.amount_msat),
                        label: format!("konduit-{timestamp}-{sequence}"),
                        description: request.description,
                    }),
            )
            .await?;
        Ok(CreatedInvoice {
            bolt11: response.bolt11,
        })
    }

    async fn get_route(&self, node_id: [u8; 33], amount_msat: u64) -> crate::Result<Route> {
        #[derive(serde::Serialize)]
        struct Request {
            id: String,
            amount_msat: String,
            riskfactor: u32,
        }

        let response: GetRouteResponse = self
            .execute(self.rpc("getroute").json(&Request {
                id: hex::encode(node_id),
                amount_msat: format!("{amount_msat}msat"),
                riskfactor: 1,
            }))
            .await?;
        let first = response.route.first().ok_or_else(|| Error::ApiError {
            status: 404,
            message: "Core Lightning found no route".into(),
        })?;
        Ok(Route {
            fee_msat: first
                .amount_msat
                .to_msat()?
                .checked_sub(amount_msat)
                .ok_or_else(|| {
                    Error::InvalidData("route amount is below destination amount".into())
                })?,
            delay_blocks: first.delay,
        })
    }

    async fn quote_route(
        &self,
        node_id: [u8; 33],
        amount_msat: u64,
        route_hint: Option<&bln_sdk::types::RouteHint>,
    ) -> crate::Result<Route> {
        if let Some(hint) = route_hint {
            let Some(first_hop) = hint.0.first() else {
                return self.get_route(node_id, amount_msat).await;
            };

            let (amount_to_hint, hint_fees, hint_delay) =
                route_hint_requirements(amount_msat, hint)?;

            let mut route = self
                .get_route(first_hop.src_node_id.serialize(), amount_to_hint)
                .await?;
            route.fee_msat = route.fee_msat.checked_add(hint_fees).ok_or(Error::Time)?;
            route.delay_blocks = route
                .delay_blocks
                .checked_add(hint_delay)
                .ok_or(Error::Time)?;
            Ok(route)
        } else {
            self.get_route(node_id, amount_msat).await
        }
    }

    fn rpc(&self, method: &str) -> reqwest::RequestBuilder {
        self.http
            .post(format!("{}/v1/{method}", self.base_url))
            .header("Rune", &self.rune)
    }

    async fn execute<T: serde::de::DeserializeOwned>(
        &self,
        request: reqwest::RequestBuilder,
    ) -> crate::Result<T> {
        let response = request.send().await?;
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await?;
            return Err(Error::ApiError { status, message });
        }
        response
            .json()
            .await
            .map_err(|error| Error::Parse(error.to_string()))
    }
}

fn route_hint_requirements(
    amount_msat: u64,
    hint: &bln_sdk::types::RouteHint,
) -> crate::Result<(u64, u64, u32)> {
    let mut amount_to_hint = amount_msat;
    let mut hint_fees = 0_u64;
    let mut hint_delay = 0_u32;
    for hop in hint.0.iter().rev() {
        let proportional = u128::from(hop.fees.proportional_millionths)
            .checked_mul(u128::from(amount_to_hint))
            .ok_or(Error::Time)?;
        let proportional = proportional.checked_add(999_999).ok_or(Error::Time)? / 1_000_000;
        let fee = u64::from(hop.fees.base_msat)
            .checked_add(u64::try_from(proportional).map_err(|_| Error::Time)?)
            .ok_or(Error::Time)?;
        amount_to_hint = amount_to_hint.checked_add(fee).ok_or(Error::Time)?;
        hint_fees = hint_fees.checked_add(fee).ok_or(Error::Time)?;
        hint_delay = hint_delay
            .checked_add(u32::from(hop.cltv_expiry_delta))
            .ok_or(Error::Time)?;
    }
    Ok((amount_to_hint, hint_fees, hint_delay))
}

#[async_trait]
impl Api for Client {
    async fn health(&self) -> crate::Result<Health> {
        Client::health(self).await
    }

    async fn create_invoice(&self, request: CreateInvoiceRequest) -> crate::Result<CreatedInvoice> {
        Client::create_invoice(self, request).await
    }

    async fn quote(&self, request: QuoteRequest) -> crate::Result<QuoteResponse> {
        let mut candidates = Vec::with_capacity(request.route_hints.len() + 1);
        if let Ok(route) = self
            .quote_route(request.payee, request.amount_msat, None)
            .await
        {
            candidates.push(route);
        }
        for hint in &request.route_hints {
            if let Ok(route) = self
                .quote_route(request.payee, request.amount_msat, Some(hint))
                .await
            {
                candidates.push(route);
            }
        }
        if candidates.is_empty() {
            return Err(Error::ApiError {
                status: 404,
                message: "Core Lightning found no route".into(),
            });
        }
        let route = candidates
            .into_iter()
            .min_by_key(|route| route.fee_msat)
            .ok_or_else(|| Error::ApiError {
                status: 404,
                message: "Core Lightning found no route".into(),
            })?;
        let max_delay_blocks = route
            .delay_blocks
            .checked_add(CLN_SHADOW_ROUTING_MARGIN_BLOCKS)
            .ok_or(Error::Time)?;
        let relative_timeout = self
            .block_time
            .checked_mul(max_delay_blocks)
            .ok_or(Error::Time)?;
        Ok(QuoteResponse {
            backend: "cln".into(),
            amount_msat: request.amount_msat,
            payee: request.payee,
            relative_timeout,
            fee_msat: route.fee_msat,
        })
    }

    async fn commit(&self, request: PayRequest) -> crate::Result<PayResponse> {
        validate_commit_quote("cln", &request.quote, &request.invoice)?;
        #[derive(serde::Serialize)]
        struct PayRpcRequest {
            bolt11: String,
            maxfee: String,
            maxdelay: u32,
        }

        #[derive(Deserialize)]
        struct PayRpcResponse {
            status: String,
            payment_preimage: Option<String>,
        }

        let blocks = request
            .quote
            .relative_timeout
            .as_secs()
            .div_ceil(self.block_time.as_secs());
        let maxdelay = u32::try_from(blocks).map_err(|_| Error::Time)?;
        let response: PayRpcResponse = self
            .execute(self.rpc("pay").json(&PayRpcRequest {
                bolt11: request.invoice.to_string(),
                maxfee: format!("{}msat", request.quote.fee_msat),
                maxdelay,
            }))
            .await?;
        if response.status != "complete" {
            return Err(Error::ApiError {
                status: 502,
                message: format!("Core Lightning payment status: {}", response.status),
            });
        }
        let preimage = response.payment_preimage.ok_or_else(|| Error::ApiError {
            status: 502,
            message: "Core Lightning completed payment without a preimage".into(),
        })?;
        let bytes = hex::decode(preimage)?;
        let secret = bytes
            .try_into()
            .map_err(|_| Error::InvalidData("payment preimage must be 32 bytes".into()))?;
        Ok(PayResponse {
            secret: Some(secret),
        })
    }

    async fn reveal(&self, request: RevealRequest) -> crate::Result<RevealResponse> {
        #[derive(serde::Serialize)]
        struct ListSendPaysRequest {
            payment_hash: String,
        }

        #[derive(Deserialize)]
        struct ListSendPaysResponse {
            payments: Vec<SentPayment>,
        }

        #[derive(Deserialize)]
        struct SentPayment {
            status: String,
            payment_preimage: Option<String>,
        }

        let response: ListSendPaysResponse = self
            .execute(self.rpc("listsendpays").json(&ListSendPaysRequest {
                payment_hash: hex::encode(request.lock),
            }))
            .await?;
        let secret = response
            .payments
            .into_iter()
            .find(|payment| payment.status == "complete")
            .and_then(|payment| payment.payment_preimage)
            .map(hex::decode)
            .transpose()?
            .map(|bytes| {
                bytes
                    .try_into()
                    .map_err(|_| Error::InvalidData("payment preimage must be 32 bytes".into()))
            })
            .transpose()?;
        Ok(RevealResponse { secret })
    }
}

#[derive(Debug)]
struct Route {
    fee_msat: u64,
    delay_blocks: u32,
}

#[derive(Debug, Deserialize)]
struct GetRouteResponse {
    route: Vec<GetRouteHop>,
}

#[derive(Debug, Deserialize)]
struct GetRouteHop {
    amount_msat: RpcAmount,
    delay: u32,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RpcAmount {
    Integer(u64),
    String(String),
}

impl RpcAmount {
    fn to_msat(&self) -> crate::Result<u64> {
        match self {
            Self::Integer(amount) => Ok(*amount),
            Self::String(amount) => {
                if let Some(msat) = amount.strip_suffix("msat") {
                    msat.parse()
                        .map_err(|error| Error::Parse(format!("invalid msat amount: {error}")))
                } else if let Some(sat) = amount.strip_suffix("sat") {
                    sat.parse::<u64>()
                        .map_err(|error| Error::Parse(format!("invalid sat amount: {error}")))?
                        .checked_mul(1000)
                        .ok_or(Error::Time)
                } else {
                    amount
                        .parse()
                        .map_err(|error| Error::Parse(format!("invalid msat amount: {error}")))
                }
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct GetInfo {
    id: Option<String>,
    alias: Option<String>,
    network: Option<String>,
    blockheight: Option<u32>,
    num_peers: Option<u32>,
    num_active_channels: Option<u32>,
    warning_bitcoind_sync: Option<String>,
    warning_lightningd_sync: Option<String>,
}

impl From<GetInfo> for Health {
    fn from(info: GetInfo) -> Self {
        Self {
            backend: "cln".into(),
            identity_pubkey: info.id,
            alias: info.alias,
            network: info.network,
            block_height: info.blockheight,
            synced_to_chain: Some(
                info.warning_bitcoind_sync
                    .is_none_or(|warning| warning.is_empty()),
            ),
            synced_to_graph: Some(
                info.warning_lightningd_sync
                    .is_none_or(|warning| warning.is_empty()),
            ),
            num_peers: info.num_peers,
            num_active_channels: info.num_active_channels,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{GetInfo, RpcAmount, route_hint_requirements};
    use crate::Health;

    #[test]
    fn maps_core_lightning_info_and_sync_warnings() {
        let info: GetInfo = serde_json::from_str(
            r#"{"id":"02abc","alias":"node","network":"testnet","blockheight":42,"num_peers":3,"num_active_channels":2,"warning_bitcoind_sync":"","warning_lightningd_sync":"not synced"}"#,
        )
        .unwrap();

        let health = Health::from(info);
        assert_eq!(health.backend, "cln");
        assert_eq!(health.identity_pubkey.as_deref(), Some("02abc"));
        assert_eq!(health.synced_to_chain, Some(true));
        assert_eq!(health.synced_to_graph, Some(false));
        assert_eq!(health.num_peers, Some(3));
        assert_eq!(health.num_active_channels, Some(2));
    }

    #[test]
    fn missing_cln_sync_warnings_mean_synced() {
        let info: GetInfo = serde_json::from_str(r#"{"id":"02abc"}"#).unwrap();
        let health = Health::from(info);
        assert_eq!(health.synced_to_chain, Some(true));
        assert_eq!(health.synced_to_graph, Some(true));
    }

    #[test]
    fn route_hint_quote_includes_forwarding_fee_and_cltv() {
        let hint: bln_sdk::types::RouteHint = serde_json::from_str(
            r#"[{"src_node_id":"0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798","short_channel_id":1,"fees":{"base_msat":100,"proportional_millionths":100},"cltv_expiry_delta":40}]"#,
        )
        .unwrap();

        assert_eq!(
            route_hint_requirements(1_000_000, &hint).unwrap(),
            (1_000_200, 200, 40)
        );
    }

    #[test]
    fn cln_amount_accepts_number_and_unit_strings() {
        assert_eq!(
            serde_json::from_str::<RpcAmount>("1000")
                .unwrap()
                .to_msat()
                .unwrap(),
            1000
        );
        assert_eq!(
            serde_json::from_str::<RpcAmount>(r#""1000msat""#)
                .unwrap()
                .to_msat()
                .unwrap(),
            1000
        );
        assert_eq!(
            serde_json::from_str::<RpcAmount>(r#""1sat""#)
                .unwrap()
                .to_msat()
                .unwrap(),
            1000
        );
    }
}
