use crate::core::{
    AdaptorInfo, Invoice, Keytag, Locked, PayBody, Quote, QuoteBody, Receipt, Squash, SquashStatus,
    Tag, TxHelp,
};
use anyhow::anyhow;
use http_client::{HeaderPolicy, Transport, codec, header_policy};

const HEADER_NAME_KEYTAG: &str = "KONDUIT";

pub type Client<T> = http_client::Client<T, codec::Json>;

pub struct Adaptor<T: Transport> {
    http_client: Client<T>,
    info: AdaptorInfo<()>,
    keytag: Option<(Tag, String)>,
}

/// An isomorphic Adaptor (a.k.a konduit-server) client that selectively pick a platform-compatible
/// http client internally. From the outside, it provides the exact same interface.
impl<T: Transport> Adaptor<T> {
    pub async fn new(http_client: Client<T>, keytag: Option<&Keytag>) -> anyhow::Result<Self> {
        let info = http_client
            .get::<AdaptorInfo<TxHelp>>("/info")
            .await
            .map_err(|e| anyhow!(e))?;

        let mut adaptor = Self {
            http_client,
            info: info.into(),
            keytag: None,
        };

        adaptor.set_keytag(keytag);

        Ok(adaptor)
    }

    fn with_keytag_header(&self) -> Vec<Box<dyn HeaderPolicy>> {
        let mut headers = vec![];

        if let Some((_, keytag)) = self.keytag.as_ref() {
            headers.push(header_policy::Custom::new(HEADER_NAME_KEYTAG, keytag).boxed());
        }

        headers
    }

    pub fn set_keytag(&mut self, keytag: Option<&Keytag>) {
        self.keytag = keytag.map(|k| {
            let (_, tag) = k.split();
            (tag, k.to_string())
        });
    }

    pub fn info(&self) -> &AdaptorInfo<()> {
        &self.info
    }

    pub fn tag(&self) -> Option<&Tag> {
        self.keytag.as_ref().map(|(tag, _)| tag)
    }

    pub fn base_url(&self) -> &str {
        self.http_client.base_url()
    }

    pub async fn receipt(&self) -> anyhow::Result<Option<Receipt>> {
        self.http_client
            .get_with_headers::<Option<Receipt>>("/ch/receipt", self.with_keytag_header())
            .await
            .map_err(|e| anyhow!(e))
    }

    pub async fn quote(&self, invoice: &Invoice) -> anyhow::Result<Quote> {
        self.http_client
            .post_with_headers::<QuoteBody, Quote>(
                "/ch/quote",
                &QuoteBody::Bolt11(invoice.clone()),
                self.with_keytag_header(),
            )
            .await
            .map_err(|e| anyhow!(e))
    }

    pub async fn pay(&self, invoice: &Invoice, locked: Locked) -> anyhow::Result<SquashStatus> {
        self.http_client
            .post_with_headers::<PayBody, SquashStatus>(
                "/ch/pay",
                &PayBody {
                    cheque_body: locked.body,
                    signature: locked.signature,
                    invoice: invoice.to_string(),
                },
                self.with_keytag_header(),
            )
            .await
            .map_err(|e| anyhow!(e))
    }

    // FIXME : This used to be cbor,
    // but everything else is json.
    // The newer http_client does not support switching between encodings.
    // Rather than hacking this back to where it was,
    // we need to fix this elsewhere: the server, and then permit the client to
    // switch between json and cbor.
    pub async fn squash(&self, squash: Squash) -> anyhow::Result<SquashStatus> {
        let mut headers = self.with_keytag_header();
        headers.push(header_policy::ContentType::from_encoder::<()>(&codec::Cbor).boxed());

        self.http_client
            .post_with_headers::<Squash, SquashStatus>("/ch/squash", &squash, headers)
            .await
            .map_err(|e| anyhow!(e))
    }
}
