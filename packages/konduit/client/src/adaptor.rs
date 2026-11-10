use crate::core::{Keytag, Locked, Tag};
use anyhow::anyhow;
use http_client::{HeaderPolicy, Transport, codec, header_policy};
use konduit_l2_dto::{info, pay, quote, receipt, squash};

const HEADER_NAME_KEYTAG: &str = "KONDUIT";

pub type Client<T> = http_client::Client<T, codec::Json>;

pub struct Adaptor<T: Transport> {
    http_client: Client<T>,
    info: info::Response<info::TxHelp>,
    keytag: Option<(Tag, String)>,
}

/// An isomorphic Adaptor (a.k.a konduit-server) client that selectively pick a platform-compatible
/// http client internally. From the outside, it provides the exact same interface.
impl<T: Transport> Adaptor<T> {
    pub async fn new(http_client: Client<T>, keytag: Option<&Keytag>) -> anyhow::Result<Self> {
        let info = http_client
            .get::<info::Response<info::TxHelp>>("/info")
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

    pub fn info(&self) -> &info::Response<info::TxHelp> {
        &self.info
    }

    pub fn tag(&self) -> Option<&Tag> {
        self.keytag.as_ref().map(|(tag, _)| tag)
    }

    pub fn base_url(&self) -> &str {
        self.http_client.base_url()
    }

    pub async fn receipt(&self) -> anyhow::Result<receipt::Response> {
        self.http_client
            .get_with_headers::<receipt::Response>(
                "/pay/konduit/receipt",
                self.with_keytag_header(),
            )
            .await
            .map_err(|e| anyhow!(e))
    }

    /// What `rail` charges to carry `amount`, konduit's own terms
    /// included. `instruction` is the rail's, and empty for the reserved
    /// `konduit` name.
    pub async fn quote(
        &self,
        rail: &str,
        amount: u64,
        instruction: Vec<u8>,
    ) -> anyhow::Result<quote::Response> {
        let request = quote::Request {
            amount,
            instruction,
        };
        self.http_client
            .post_with_headers::<quote::Request, quote::Response>(
                &format!("/pay/{rail}/quote"),
                &request,
                self.with_keytag_header(),
            )
            .await
            .map_err(|e| anyhow!(e))
    }

    pub async fn pay(
        &self,
        rail: &str,
        inbound: Locked,
        instruction: Vec<u8>,
    ) -> anyhow::Result<pay::Response> {
        let request = pay::Request {
            inbound,
            instruction,
        };
        self.http_client
            .post_with_headers::<pay::Request, pay::Response>(
                &format!("/pay/{rail}/pay"),
                &request,
                self.with_keytag_header(),
            )
            .await
            .map_err(|e| anyhow!(e))
    }

    /// Where the inbound leg stands: what konduit expects signed next,
    /// and the unlocked cheques a pending payment has resolved into.
    pub async fn squash_status(&self) -> anyhow::Result<squash::Response> {
        self.http_client
            .get_with_headers::<squash::Response>("/pay/konduit/squash", self.with_keytag_header())
            .await
            .map_err(|e| anyhow!(e))
    }

    // FIXME : This used to be cbor,
    // but everything else is json.
    // The newer http_client does not support switching between encodings.
    // Rather than hacking this back to where it was,
    // we need to fix this elsewhere: the server, and then permit the client to
    // switch between json and cbor.
    pub async fn squash(&self, squash: squash::Request) -> anyhow::Result<squash::Response> {
        let headers = self.with_keytag_header();

        let res = self
            .http_client
            .post_with_headers::<squash::Request, squash::Response>(
                "/pay/konduit/squash",
                &squash,
                headers,
            )
            .await;
        res.map_err(|e| anyhow!(e))
    }
}
