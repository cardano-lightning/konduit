use crate::core::{Invoice, Keytag, Locked, Squash, Tag, wire};
use anyhow::anyhow;
use cardano_sdk::Signature;
use http_client::{CborCodec, HeaderPolicy, HttpClient, HttpTransport, JsonCodec};

const HEADER_CONTENT_TYPE_JSON: (&str, &str) = ("Content-Type", "application/json");
const HEADER_CONTENT_TYPE_CBOR: (&str, &str) = ("Content-Type", "application/cbor");
const HEADER_NAME_KEYTAG: &str = "KONDUIT";

pub type HttpCbor<T> = http_client::HttpClient<T, CborCodec>;
pub type HttpJson<T> = http_client::HttpClient<T, JsonCodec>;

pub struct Adaptor<H: HttpTransport, C> {
    http_client: HttpClient<H, C>,
    signer: fn(v: &Vec<u8>) -> Signature,
    credential: Option<wire::reg::cobbl3::Credential>,
}

#[derive(Debug, Clone)]
pub enum MediaType {
    Cbor,
    Json,
}

impl Default for MediaType {
    fn default() -> Self {
        Self::Cbor
    }
}

/// An isomorphic Adaptor (a.k.a konduit-server) client that selectively pick a platform-compatible
/// http client internally. From the outside, it provides the exact same interface.
impl<H: HttpTransport, C> Adaptor<H, C> {
    pub fn new(http_client: HttpClient<H, C>, signer: fn(v: &Vec<u8>) -> Signature) -> Self {
        Self {
            http_client,
            signer,
            credential: None,
        }
    }

    // fn auth_header() -> Vec<(&'static str, &'a str)> {
    //
    // }

    // fn with_keytag_header<'a>(
    //     &'a self,
    //     others: &[(&'static str, &'a str)],
    // ) -> Vec<(&'static str, &'a str)> {
    //     let mut headers = Vec::from(others);

    //     if let Some((_, keytag)) = self.keytag.as_ref() {
    //         headers.push((HEADER_NAME_KEYTAG, keytag));
    //     }

    //     headers
    // }

    // pub fn set_keytag(&mut self, keytag: Option<&Keytag>) {
    //     self.keytag = keytag.map(|k| {
    //         let (_, tag) = k.split();
    //         (tag, k.to_string())
    //     });
    // }

    // pub fn info(&self) -> &wire::info::Response {
    //     &self.info
    // }

    // pub fn tag(&self) -> Option<&Tag> {
    //     self.keytag.as_ref().map(|(tag, _)| tag)
    // }

    // pub fn base_url(&self) -> &str {
    //     self.http_client.base_url()
    // }

    // pub async fn reg(&self, body: &wire::reg::cobbl3::Body) -> anyhow::Result<wire::reg::cobbl3::Response> {
    //     self.http_client
    //         .post_with_headers::<wire::reg::cobbl3::Body, wire::reg::cobbl3::Response>(
    //             wire::auth::state::PATH,
    //             &self.with_keytag_header(&[]),
    //             body
    //         )
    //         .await
    //         .map_err(|e| anyhow!(e))
    // }

    // pub async fn state(&self) -> anyhow::Result<wire::auth::state::Response> {
    //     self.http_client
    //         .get_with_headers::<wire::auth::state::Response>(wire::auth::state::PATH, &self.with_keytag_header(&[]))
    //         .await
    //         .map_err(|e| anyhow!(e))
    // }

    // pub async fn quote(&self, invoice: &Invoice) -> anyhow::Result<wire::auth::pay::bolt11::quote::Response> {
    //     self.http_client
    //             .post_with_headers::<wire::auth::pay::bolt11::quote::Body, wire::auth::pay::bolt11::quote::Response>(
    //             wire::auth::pay::bolt11::quote::PATH,
    //             &self.with_keytag_header(&[HEADER_CONTENT_TYPE_JSON]),
    //             &invoice.to_string(),
    //         )
    //         .await
    //         .map_err(|e| anyhow!(e))
    // }

    // pub async fn commit(&self, body : wire::auth::pay::bolt11::commit::Body) -> anyhow::Result<wire::auth::pay::bolt11::commit::Response> {
    //     self.http_client
    //             .post_with_headers::<wire::auth::pay::bolt11::commit::Body, wire::auth::pay::bolt11::commit::Response>(
    //             wire::auth::pay::bolt11::commit::PATH,
    //             &self.with_keytag_header(&[HEADER_CONTENT_TYPE_JSON]),
    //             &body,
    //         )
    //         .await
    //         .map_err(|e| anyhow!(e))
    // }

    // pub async fn post_auth<Body, Response>(&self, path: &str, body : &Body) -> anyhow::Result<Response> {
    //     let headers = self.auth_headers();
    //     self.http_client
    //             .post_with_headers::<Body, Response>(
    //             path,
    //             headers,
    //             body,
    //         )
    //         .await
    //         .map_err(|e| anyhow!(e))
    // }

    // // FIXME : This used to be cbor,
    // // but everything else is json.
    // // The newer http_client does not support switching between encodings.
    // // Rather than hacking this back to where it was,
    // // we need to fix this elsewhere: the server, and then permit the client to
    // // switch between json and cbor.
    // pub async fn squash(&self, body: wire::auth::squash::Body) -> anyhow::Result<wire::auth::squash::Response> {
    //     self.http_client
    //         .post_with_headers::<wire::auth::squash::Body, wire::auth::squash::Response>(
    //             wire::auth::squash::PATH,
    //             &self.with_keytag_header(&[HEADER_CONTENT_TYPE_CBOR]),
    //             &body,
    //         )
    //         .await
    //         .map_err(|e| anyhow!(e))
    // }
}
