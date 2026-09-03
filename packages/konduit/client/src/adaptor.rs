use crate::core::{
    AdaptorInfo, Invoice, Keytag, Locked, PayBody, Quote, QuoteBody, Receipt, SessionClaimRequest,
    SessionClaimResponse, Squash, SquashStatus, Tag, TxHelp,
};
use anyhow::anyhow;
use http_client::{HeaderPolicy, Transport, codec, header_policy};
use std::sync::Mutex;

const HEADER_NAME_KEYTAG: &str = "KONDUIT";
const HEADER_NAME_SESSION: &str = "FERRET-SESSION";

pub type Client<T> = http_client::Client<T, codec::Json>;

pub struct Adaptor<T: Transport> {
    http_client: Client<T>,
    info: AdaptorInfo<TxHelp>,
    keytag: Option<(Tag, String)>,
    session: Mutex<Option<SessionClaimResponse>>,
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
            info,
            keytag: None,
            session: Mutex::new(None),
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

    fn mutation_headers(&self) -> anyhow::Result<Vec<Box<dyn HeaderPolicy>>> {
        let session = self.session.lock().expect("session lock");
        let lease = session
            .as_ref()
            .ok_or_else(|| anyhow!("missing FERRET-SESSION lease"))?
            .lease
            .clone();
        drop(session);
        let mut headers = self.with_keytag_header();
        headers.push(header_policy::Custom::new(HEADER_NAME_SESSION, &lease).boxed());
        Ok(headers)
    }

    pub fn set_keytag(&mut self, keytag: Option<&Keytag>) {
        self.keytag = keytag.map(|k| {
            let (_, tag) = k.split();
            (tag, k.to_string())
        });
    }

    pub async fn claim_session(
        &self,
        claim: &SessionClaimRequest,
    ) -> anyhow::Result<SessionClaimResponse> {
        let session = self
            .http_client
            .post::<SessionClaimRequest, SessionClaimResponse>("/session/claim", claim)
            .await
            .map_err(|error| anyhow!(error))?;
        if !valid_lease(&session.lease) {
            return Err(anyhow!("invalid FERRET-SESSION lease"));
        }
        *self.session.lock().expect("session lock") = Some(session.clone());
        Ok(session)
    }

    pub fn info(&self) -> &AdaptorInfo<TxHelp> {
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
                self.mutation_headers()?,
            )
            .await
            .map_err(|e| anyhow!(e))
    }

    pub async fn pay(&self, invoice: &Invoice, locked: Locked) -> anyhow::Result<SquashStatus> {
        self.http_client
            .post_with_headers::<PayBody, SquashStatus>(
                "/ch/pay",
                &PayBody {
                    cheque_body: locked.body().to_owned(),
                    signature: locked.signature().to_owned(),
                    invoice: invoice.clone(),
                },
                self.mutation_headers()?,
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
        let headers = self.mutation_headers()?;

        let res = self
            .http_client
            .post_with_headers::<Squash, SquashStatus>("/ch/squash", &squash, headers)
            .await;
        res.map_err(|e| anyhow!(e))
    }
}

fn valid_lease(lease: &str) -> bool {
    lease.len() == 64
        && lease
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{ChannelParameters, TosInfo, VerificationKey};
    use std::convert::Infallible;

    struct NoTransport;

    impl Transport for NoTransport {
        type Error = Infallible;

        async fn transport(
            &self,
            _req: http::Request<Vec<u8>>,
        ) -> Result<http::Response<Vec<u8>>, Self::Error> {
            unreachable!()
        }
    }

    #[test]
    fn mutation_without_claim_fails_locally() {
        let adaptor = Adaptor {
            http_client: Client::new(NoTransport, codec::Json, "http://localhost".into()),
            info: AdaptorInfo {
                tos: TosInfo { flat_fee: 0 },
                channel_parameters: ChannelParameters {
                    adaptor_key: VerificationKey::from([0; 32]),
                    close_period: konduit_data::Duration::from_secs(0),
                    tag_length: 0,
                },
                tx_help: (),
                asset_catalog_digest: None,
            },
            keytag: None,
            session: Mutex::new(None),
        };

        assert_eq!(
            adaptor.mutation_headers().err().unwrap().to_string(),
            "missing FERRET-SESSION lease"
        );
    }

    #[test]
    fn lease_must_be_lowercase_hex() {
        assert!(valid_lease(&"ab".repeat(32)));
        assert!(!valid_lease(&"AB".repeat(32)));
        assert!(!valid_lease(&"ab".repeat(31)));
    }
}
