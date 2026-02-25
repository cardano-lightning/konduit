use cardano_connector_client::wasm::{
    self,
    helpers::{json_stringify, singleton, to_js_object},
};
use cardano_sdk::{PlutusData, VerificationKey, cbor::ToCbor};
use http_client::{HttpClient as _, wasm::HttpClient};
use konduit_data::{Locked, Receipt, Squash, SquashProposal, Tag};
use std::{ops::Deref, str::FromStr};
use wasm_bindgen::prelude::*;
use web_time::Duration;

#[wasm_bindgen]
pub struct Adaptor {
    http_client: HttpClient,
    info: AdaptorInfo,
}

// TODO: resolve Info duplication with konduit-server
//
// We cannot depend on the konduit-server here, but that type shall be shared between both.
#[wasm_bindgen]
#[derive(Debug, Clone, Copy)]
pub struct AdaptorInfo {
    #[wasm_bindgen(js_name = "verificationKey")]
    pub verification_key: VerificationKey,
    #[wasm_bindgen(js_name = "closePeriod")]
    pub close_period_secs: u64,
    #[wasm_bindgen(js_name = "maxTagLength")]
    pub max_tag_length: u8,
    #[wasm_bindgen(js_name = "fee")]
    pub fee: u64,
}

#[wasm_bindgen]
impl Adaptor {
    #[wasm_bindgen(js_name = "create")]
    pub async fn new(url: &str) -> wasm::Result<Self> {
        let http_client = HttpClient::new(
            url.strip_suffix("/").unwrap_or(url).to_string(),
            Duration::from_secs(10),
        );

        let info = http_client.get::<AdaptorInfo>("/info").await?;

        Ok(Self { http_client, info })
    }

    #[wasm_bindgen(getter, js_name = "url")]
    pub fn url(&self) -> String {
        self.http_client.base_url.clone()
    }

    #[wasm_bindgen(getter, js_name = "info")]
    pub fn info(&self) -> AdaptorInfo {
        self.info
    }
}

impl Adaptor {
    pub async fn receipt(
        &self,
        consumer_key: &VerificationKey,
        channel_tag: &Tag,
    ) -> wasm::Result<ReceiptResponse> {
        let key_tag = format!("{consumer_key}{channel_tag}");
        Ok(self
            .http_client
            .get_with_headers::<Option<Receipt>>("/ch/receipt", &[("konduit", &key_tag)])
            .await
            .map(ReceiptResponse)?)
    }

    pub async fn quote(
        &self,
        invoice: &str,
        consumer_key: &VerificationKey,
        channel_tag: &Tag,
    ) -> wasm::Result<QuoteResponse> {
        let key_tag = format!("{consumer_key}{channel_tag}");
        let invoice = json_stringify(singleton("Bolt11", invoice)?)?;
        Ok(self
            .http_client
            .post_with_headers::<QuoteResponse>(
                "/ch/quote",
                &[("konduit", &key_tag), ("Content-Type", "application/json")],
                invoice.as_bytes(),
            )
            .await?)
    }

    pub async fn pay(
        &self,
        invoice: &str,
        locked: Locked,
        consumer_key: &VerificationKey,
        channel_tag: &Tag,
    ) -> wasm::Result<SquashResponse> {
        let key_tag = format!("{consumer_key}{channel_tag}");

        let data = json_stringify(to_js_object(&[
            ("invoice", invoice.into()),
            (
                "cheque_body",
                hex::encode(PlutusData::from(locked.body).to_cbor()).into(),
            ),
            ("signature", hex::encode(locked.signature.as_ref()).into()),
        ])?)?;

        Ok(self
            .http_client
            .post_with_headers::<SquashResponse>(
                "/ch/pay",
                &[("konduit", &key_tag), ("Content-Type", "application/json")],
                data.as_bytes(),
            )
            .await?)
    }

    pub async fn squash(
        &self,
        squash: Squash,
        consumer_key: &VerificationKey,
        channel_tag: &Tag,
    ) -> wasm::Result<SquashResponse> {
        let key_tag = format!("{consumer_key}{channel_tag}");
        let data = PlutusData::from(squash).to_cbor();
        Ok(self
            .http_client
            .post_with_headers::<SquashResponse>(
                "/ch/squash",
                &[("konduit", &key_tag), ("Content-Type", "application/cbor")],
                data.as_slice(),
            )
            .await?)
    }
}

impl<'de> serde::Deserialize<'de> for AdaptorInfo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        pub struct RawInfo {
            channel_parameters: ChannelParameters,
            tos: TosInfo,
        }

        #[derive(serde::Deserialize)]
        pub struct ChannelParameters {
            adaptor_key: String,
            close_period: Duration,
            tag_length: u8,
        }

        #[derive(serde::Deserialize)]
        pub struct TosInfo {
            flat_fee: u64,
        }

        let info: RawInfo = RawInfo::deserialize(deserializer)?;

        let verification_key =
            VerificationKey::from_str(info.channel_parameters.adaptor_key.as_str())
                .map_err(serde::de::Error::custom)?;

        Ok(Self {
            verification_key,
            close_period_secs: info.channel_parameters.close_period.as_secs(),
            max_tag_length: info.channel_parameters.tag_length,
            fee: info.tos.flat_fee,
        })
    }
}

#[wasm_bindgen]
#[derive(Debug)]
pub struct ReceiptResponse(Option<Receipt>);

impl Deref for ReceiptResponse {
    type Target = Option<Receipt>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// TODO: resolve duplication with konduit-client
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SquashResponse {
    Complete,
    Incomplete(SquashProposal),
    Stale(SquashProposal),
}

// TODO: resolve duplication with konduit-client
#[wasm_bindgen]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QuoteResponse {
    pub index: u64,
    pub amount: u64,
    pub relative_timeout: u64,
    pub routing_fee: u64,
}
