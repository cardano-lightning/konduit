use crate::{
    HttpClient,
    core::{
        AdaptorInfo, Invoice, Keytag, Locked, PayBody, PlutusData, Quote, QuoteBody, Receipt,
        Squash, SquashStatus, cbor::ToCbor,
    },
};
use http_client::HttpClient as _;

const HEADER_CONTENT_TYPE_JSON: (&str, &str) = ("Content-Type", "application/json");
const HEADER_CONTENT_TYPE_CBOR: (&str, &str) = ("Content-Type", "application/cbor");
const HEADER_NAME_KEYTAG: &str = "KONDUIT";

pub struct Adaptor {
    http_client: HttpClient,
    info: AdaptorInfo,
    keytag: String,
}

/// An isomorphic Adaptor (a.k.a konduit-server) client that selectively pick a platform-compatible
/// http client internally. From the outside, it provides the exact same interface.
impl Adaptor {
    pub async fn new(base_url: &str, keytag: &Keytag) -> anyhow::Result<Self> {
        let http_client = HttpClient::new(base_url);
        let info = http_client.get::<AdaptorInfo>("/info").await?;
        Ok(Self {
            http_client,
            info,
            keytag: keytag.to_string(),
        })
    }

    pub fn info(&self) -> &AdaptorInfo {
        &self.info
    }
}

impl Adaptor {
    fn header_keytag(&self) -> (&'static str, &str) {
        (HEADER_NAME_KEYTAG, self.keytag.as_str())
    }
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
                HttpClient::to_json(&QuoteBody::Bolt11(invoice)),
            )
            .await
    }

    pub async fn pay(&self, invoice: &str, locked: Locked) -> anyhow::Result<SquashStatus> {
        self.http_client
            .post_with_headers::<SquashStatus>(
                "/ch/pay",
                &[self.header_keytag(), HEADER_CONTENT_TYPE_JSON],
                HttpClient::to_json(&PayBody {
                    cheque_body: locked.body,
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

#[cfg(feature = "wasm")]
pub mod wasm {
    use crate::{
        HttpClient,
        core::wasm::{self, AdaptorInfo, Keytag},
        wasm_proxy,
    };
    use wasm_bindgen::prelude::*;

    wasm_proxy! {
        /// A facade for the Adaptor server.
        Adaptor
    }

    impl Clone for Adaptor {
        fn clone(&self) -> Self {
            Self(super::Adaptor {
                http_client: HttpClient::new(&self.http_client.base_url),
                info: self.info.clone(),
                keytag: self.keytag.clone(),
            })
        }
    }

    #[wasm_bindgen]
    impl Adaptor {
        #[wasm_bindgen(js_name = "new")]
        pub async fn _wasm_new(base_url: &str, keytag: &Keytag) -> wasm::Result<Self> {
            Ok(Self::from(super::Adaptor::new(base_url, keytag).await?))
        }

        #[wasm_bindgen(getter, js_name = "info")]
        pub fn _wasm_info(&self) -> AdaptorInfo {
            AdaptorInfo::from(self.info().clone())
        }
    }
}
