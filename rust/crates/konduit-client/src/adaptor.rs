use bln_sdk::types::Invoice;
use cardano_sdk::{PlutusData, cbor::ToCbor};
use http_client::HttpClient as _;
use konduit_data::{Locked, PayBody, Quote, QuoteBody, Receipt, Squash, SquashStatus};

#[cfg(feature = "reqwest")]
use _reqwest::*;
#[cfg(feature = "reqwest")]
mod _reqwest {
    pub use anyhow::Result;
    pub use http_client::reqwest::HttpClient;
    pub use konduit_data::{AdaptorInfo, Keytag};
}

#[cfg(feature = "wasm")]
use _wasm::*;
#[cfg(feature = "wasm")]
mod _wasm {
    pub use cardano_sdk::wasm::Result;
    pub use http_client::wasm::HttpClient;
    pub use konduit_data::wasm::{AdaptorInfo, Keytag};
    pub use wasm_bindgen::prelude::*;
}

const HEADER_CONTENT_TYPE_JSON: (&str, &str) = ("Content-Type", "application/json");
const HEADER_CONTENT_TYPE_CBOR: (&str, &str) = ("Content-Type", "application/cbor");

#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub struct Adaptor {
    http_client: HttpClient,
    info: AdaptorInfo,
    keytag: (Keytag, String),
}

impl Adaptor {
    const HEADER_NAME_KEYTAG: &str = "KONDUIT";

    fn header_keytag(&self) -> (&'static str, &str) {
        (Self::HEADER_NAME_KEYTAG, self.keytag.1.as_str())
    }
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl Adaptor {
    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn info(&self) -> AdaptorInfo {
        self.info.clone()
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub async fn new(base_url: &str, keytag: &Keytag) -> Result<Self> {
        let http_client = HttpClient::new(base_url);
        let info = http_client.get::<AdaptorInfo>("/info").await?;
        Ok(Self {
            http_client,
            info,
            // NOTE: keeping an owned string to allow referencing it later.
            keytag: (keytag.clone(), keytag.to_string()),
        })
    }
}

impl Adaptor {
    pub async fn receipt(&self) -> anyhow::Result<Option<Receipt>> {
        self.http_client
            .get_with_headers::<Option<Receipt>>("/ch/receipt", &[self.header_keytag()])
            .await
    }

    pub async fn quote(&self, invoice: Invoice) -> anyhow::Result<Quote> {
        self.http_client
            .post_with_headers::<Quote>(
                "/ch/quote",
                &[self.header_keytag(), HEADER_CONTENT_TYPE_JSON],
                HttpClient::to_json(QuoteBody::Bolt11(invoice)),
            )
            .await
    }

    pub async fn pay(&self, invoice: &str, locked: &Locked) -> anyhow::Result<SquashStatus> {
        self.http_client
            .post_with_headers::<SquashStatus>(
                "/ch/pay",
                &[self.header_keytag(), HEADER_CONTENT_TYPE_JSON],
                HttpClient::to_json(PayBody {
                    cheque_body: locked.body.clone(),
                    signature: locked.signature,
                    invoice: invoice.to_string(),
                }),
            )
            .await
    }

    pub async fn squash(&self, squash: Squash) -> anyhow::Result<SquashStatus> {
        self.http_client
            .post_with_headers::<SquashStatus>(
                "/ch/squash",
                &[self.header_keytag(), HEADER_CONTENT_TYPE_CBOR],
                PlutusData::from(squash).to_cbor(),
            )
            .await
    }
}
