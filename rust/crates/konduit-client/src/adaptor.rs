use crate::core::{
    AdaptorInfo, Invoice, Keytag, Locked, PayBody, PlutusData, Quote, QuoteBody, Receipt, Squash,
    SquashStatus, cbor::ToCbor,
};
use anyhow::anyhow;
use http_client::HttpClient;

const HEADER_CONTENT_TYPE_JSON: (&str, &str) = ("Content-Type", "application/json");
const HEADER_CONTENT_TYPE_CBOR: (&str, &str) = ("Content-Type", "application/cbor");
const HEADER_NAME_KEYTAG: &str = "KONDUIT";

pub struct Adaptor<Http: HttpClient> {
    http_client: Http,
    info: AdaptorInfo,
    keytag: String,
}

/// An isomorphic Adaptor (a.k.a konduit-server) client that selectively pick a platform-compatible
/// http client internally. From the outside, it provides the exact same interface.
impl<Http: HttpClient> Adaptor<Http>
where
    Http::Error: Into<anyhow::Error>,
{
    pub async fn new(http_client: Http, keytag: &Keytag) -> anyhow::Result<Self> {
        let info = http_client
            .get::<AdaptorInfo>("/info")
            .await
            .map_err(|e| anyhow!(e))?;
        Ok(Self {
            http_client,
            info,
            keytag: keytag.to_string(),
        })
    }

    fn header_keytag(&self) -> (&'static str, &str) {
        (HEADER_NAME_KEYTAG, self.keytag.as_str())
    }

    pub fn info(&self) -> &AdaptorInfo {
        &self.info
    }

    pub async fn receipt(&self) -> anyhow::Result<Option<Receipt>> {
        self.http_client
            .get_with_headers::<Option<Receipt>>("/ch/receipt", &[self.header_keytag()])
            .await
            .map_err(|e| anyhow!(e))
    }

    pub async fn quote(&self, invoice: Invoice) -> anyhow::Result<Quote> {
        self.http_client
            .post_with_headers::<Quote>(
                "/ch/quote",
                &[self.header_keytag(), HEADER_CONTENT_TYPE_JSON],
                Http::to_json(&QuoteBody::Bolt11(invoice)),
            )
            .await
            .map_err(|e| anyhow!(e))
    }

    pub async fn pay(&self, invoice: &str, locked: Locked) -> anyhow::Result<SquashStatus> {
        self.http_client
            .post_with_headers::<SquashStatus>(
                "/ch/pay",
                &[self.header_keytag(), HEADER_CONTENT_TYPE_JSON],
                Http::to_json(&PayBody {
                    cheque_body: locked.body,
                    signature: locked.signature,
                    invoice: invoice.to_string(),
                }),
            )
            .await
            .map_err(|e| anyhow!(e))
    }

    pub async fn squash(&self, squash: Squash) -> anyhow::Result<SquashStatus> {
        self.http_client
            .post_with_headers::<SquashStatus>(
                "/ch/squash",
                &[self.header_keytag(), HEADER_CONTENT_TYPE_CBOR],
                PlutusData::from(squash).to_cbor(),
            )
            .await
            .map_err(|e| anyhow!(e))
    }
}

#[cfg(feature = "wasm")]
pub mod wasm {
    use crate::{
        core::wasm::{self, AdaptorInfo, Keytag},
        wasm_proxy,
    };
    use http_client_wasm::HttpClient;
    use wasm_bindgen::prelude::*;

    wasm_proxy! {
        #[doc = "A facade for the Adaptor server."]
        Adaptor => super::Adaptor<HttpClient>
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
            Ok(Self::from(
                super::Adaptor::new(HttpClient::new(base_url), keytag).await?,
            ))
        }

        #[wasm_bindgen(getter, js_name = "info")]
        pub fn _wasm_info(&self) -> AdaptorInfo {
            AdaptorInfo::from(self.info().clone())
        }
    }
}
