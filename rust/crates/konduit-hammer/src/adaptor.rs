use std::iter;

use crate::core::{
    AdaptorInfo, Invoice, Keytag, Locked, PayBody, PlutusData, Quote, QuoteBody, Receipt, Squash,
    SquashStatus, TxHelp, cbor::ToCbor,
};
use anyhow::anyhow;
use http_client::HttpClient;
use serde::de::DeserializeOwned;

const HEADER_CONTENT_TYPE_JSON: (&str, &str) = ("Content-Type", "application/json");
const HEADER_CONTENT_TYPE_CBOR: (&str, &str) = ("Content-Type", "application/cbor");
const HEADER_NAME_KEYTAG: &str = "KONDUIT";

pub struct Adaptor<Http: HttpClient> {
    http_client: Http,
}

/// An adaptor client cribbed from konduit-client,
/// but augmented to be indendent of keytag.
impl<Http: HttpClient> Adaptor<Http>
where
    Http::Error: Into<anyhow::Error>,
{
    pub fn new(http_client: Http) -> Self {
        Self { http_client }
    }

    pub fn base_url(&self) -> &str {
        self.http_client.base_url()
    }

    pub async fn info(&self) -> anyhow::Result<AdaptorInfo<TxHelp>> {
        self.http_client
            .get::<AdaptorInfo<TxHelp>>("/info")
            .await
            .map_err(|e| anyhow!(e))
    }

    pub async fn get_with_headers<Res: DeserializeOwned>(
        &self,
        path: &str,
        headers: &[(&str, &str)],
    ) -> anyhow::Result<Res> {
        self.http_client
            .get_with_headers::<Res>(path, headers)
            .await
            .map_err(|e| anyhow!(e))
    }

    pub async fn post_with_headers<Res: DeserializeOwned, Body: AsRef<[u8]>>(
        &self,
        path: &str,
        headers: &[(&str, &str)],
        body: Body,
    ) -> anyhow::Result<Res> {
        self.http_client
            .post_with_headers::<Res>(path, headers, body)
            .await
            .map_err(|e| anyhow!(e))
    }

    pub async fn get_ch<Res: DeserializeOwned>(
        &self,
        path: &str,
        keytag: &Keytag,
        other_headers: Vec<(&str, &str)>,
    ) -> anyhow::Result<Res> {
        self.get_with_headers::<Res>(
            format!("/ch{}", path).as_str(),
            with_keytag_header(&keytag.to_string(), other_headers).as_slice(),
        )
        .await
    }

    pub async fn post_ch<Res: DeserializeOwned, Body: AsRef<[u8]>>(
        &self,
        path: &str,
        keytag: &Keytag,
        other_headers: Vec<(&str, &str)>,
        body: Body,
    ) -> anyhow::Result<Res> {
        self.post_with_headers::<Res, Body>(
            format!("/ch{}", path).as_str(),
            with_keytag_header(&keytag.to_string(), other_headers).as_slice(),
            body,
        )
        .await
    }

    pub async fn receipt(&self, keytag: &Keytag) -> anyhow::Result<Option<Receipt>> {
        self.get_ch::<Option<Receipt>>("/receipt", &keytag, vec![])
            .await
    }

    pub async fn quote(&self, keytag: &Keytag, invoice: &Invoice) -> anyhow::Result<Quote> {
        self.post_ch::<Quote, Vec<u8>>(
            "/quote",
            keytag,
            vec![HEADER_CONTENT_TYPE_JSON],
            Http::to_json(&QuoteBody::Bolt11(invoice.clone())),
        )
        .await
    }

    pub async fn pay(
        &self,
        keytag: &Keytag,
        invoice: &Invoice,
        locked: Locked,
    ) -> anyhow::Result<SquashStatus> {
        self.post_ch::<SquashStatus, Vec<u8>>(
            "/pay",
            keytag,
            vec![HEADER_CONTENT_TYPE_JSON],
            Http::to_json(&PayBody {
                cheque_body: locked.body,
                signature: locked.signature,
                invoice: invoice.to_string(),
            }),
        )
        .await
    }

    pub async fn squash(&self, keytag: &Keytag, squash: Squash) -> anyhow::Result<SquashStatus> {
        self.post_ch::<SquashStatus, Vec<u8>>(
            "/squash",
            keytag,
            vec![HEADER_CONTENT_TYPE_CBOR],
            PlutusData::from(squash).to_cbor(),
        )
        .await
    }
}

fn with_keytag_header<'a>(
    keytag_string: &'a String,
    other_headers: Vec<(&'a str, &'a str)>,
) -> Vec<(&'a str, &'a str)> {
    iter::once((HEADER_NAME_KEYTAG, keytag_string.as_str()))
        .chain(other_headers.into_iter())
        .collect::<Vec<_>>()
}
