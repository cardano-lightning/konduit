use crate::core::{
    AdaptorInfo, Invoice, Keytag, Locked, PayBody, PlutusData, Quote, QuoteBody, Receipt, Squash,
    SquashStatus, Tag, cbor::ToCbor,
};
use anyhow::anyhow;
use http_client::HttpClient;

const HEADER_CONTENT_TYPE_JSON: (&str, &str) = ("Content-Type", "application/json");
const HEADER_CONTENT_TYPE_CBOR: (&str, &str) = ("Content-Type", "application/cbor");
const HEADER_NAME_KEYTAG: &str = "KONDUIT";

pub struct Adaptor<Http: HttpClient> {
    http_client: Http,
    info: AdaptorInfo,
    keytag: Option<(Tag, String)>,
}

/// An isomorphic Adaptor (a.k.a konduit-server) client that selectively pick a platform-compatible
/// http client internally. From the outside, it provides the exact same interface.
impl<Http: HttpClient> Adaptor<Http>
where
    Http::Error: Into<anyhow::Error>,
{
    pub async fn new(http_client: Http, keytag: Option<&Keytag>) -> anyhow::Result<Self> {
        let info = http_client
            .get::<AdaptorInfo>("/info")
            .await
            .map_err(|e| anyhow!(e))?;

        let mut adaptor = Self {
            http_client,
            info,
            keytag: None,
        };

        adaptor.set_keytag(keytag);

        Ok(adaptor)
    }

    fn with_keytag_header<'a>(
        &'a self,
        others: &[(&'static str, &'a str)],
    ) -> Vec<(&'static str, &'a str)> {
        let mut headers = Vec::from(others);

        if let Some((_, keytag)) = self.keytag.as_ref() {
            headers.push((HEADER_NAME_KEYTAG, keytag));
        }

        headers
    }

    pub fn set_keytag(&mut self, keytag: Option<&Keytag>) {
        self.keytag = keytag.map(|k| {
            let (_, tag) = k.split();
            (tag, k.to_string())
        });
    }

    pub fn info(&self) -> &AdaptorInfo {
        &self.info
    }

    pub fn tag(&self) -> Option<&Tag> {
        self.keytag.as_ref().map(|(tag, _)| tag)
    }

    pub async fn receipt(&self) -> anyhow::Result<Option<Receipt>> {
        self.http_client
            .get_with_headers::<Option<Receipt>>("/ch/receipt", &self.with_keytag_header(&[]))
            .await
            .map_err(|e| anyhow!(e))
    }

    pub async fn quote(&self, invoice: Invoice) -> anyhow::Result<Quote> {
        self.http_client
            .post_with_headers::<Quote>(
                "/ch/quote",
                &self.with_keytag_header(&[HEADER_CONTENT_TYPE_JSON]),
                Http::to_json(&QuoteBody::Bolt11(invoice)),
            )
            .await
            .map_err(|e| anyhow!(e))
    }

    pub async fn pay(&self, invoice: &str, locked: Locked) -> anyhow::Result<SquashStatus> {
        self.http_client
            .post_with_headers::<SquashStatus>(
                "/ch/pay",
                &self.with_keytag_header(&[HEADER_CONTENT_TYPE_JSON]),
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
                &self.with_keytag_header(&[HEADER_CONTENT_TYPE_CBOR]),
                PlutusData::from(squash).to_cbor(),
            )
            .await
            .map_err(|e| anyhow!(e))
    }
}
