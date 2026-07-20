//! Konduit server client.
//!
//! This is a very thin wrapper of http_client,
//! plumbing in the pieces from konduit-wire so that it talks to konduit-server.
//! Beyond the http client, it owns no state:
//! Tag, credentials, _etc_ are caller's responsibility
//!
use http_client::{Decoder, Encoder, Transport, client};
use problem_details::ProblemDetailBody;

use konduit_wire as wire;

use super::{Codec, Config, Error, codec};

pub struct Client<T, C> {
    inner: http_client::Client<T, C>,
}

impl<T: Transport, C> Client<T, C> {
    pub fn new(transport: T, codec: C, base_url: &str) -> Self {
        Self {
            inner: http_client::Client::new(transport, codec, base_url.to_owned()),
        }
    }

    async fn send<Req, Res>(
        &self,
        method: http::Method,
        path: &str,
        credential: Option<&str>,
        body: Option<&Req>,
    ) -> Result<Res, Error>
    where
        C: Codec + Encoder<Req> + Decoder<Res>,
        <C as Codec>::Error: From<<C as Encoder<Req>>::Error> + From<<C as Decoder<Res>>::Error>,
    {
        let mut rb = self.inner.request(method, path);
        if let Some(cred) = credential {
            rb = rb.map_builder(|b| {
                b.header(
                    http::header::AUTHORIZATION,
                    format!("{} {cred}", wire::reg::cobbl3::SCHEME),
                )
            });
        }

        match rb.send::<Req, Res>(body).await {
            Ok(res) => Ok(res),
            Err(client::Error::Transport(t)) => Err(Error::Transport(Box::new(t))),
            Err(client::Error::Encode(e)) => {
                Err(Error::Codec(Box::new(<C as Codec>::Error::from(e))))
            }
            Err(client::Error::Decode(e)) => {
                Err(Error::Codec(Box::new(<C as Codec>::Error::from(e))))
            }
            Err(client::Error::Http(e)) => Err(Error::Http(e)),
            Err(client::Error::Response { status, body }) => Err(if status.is_client_error() {
                match self.inner.decode::<ProblemDetailBody>(&body) {
                    Ok(problem) => Error::Problem { status, problem },
                    Err(_) => Error::ProblemUnparsed {
                        status,
                        raw_body: body,
                    },
                }
            } else {
                Error::ServerStatus {
                    status,
                    raw_body: body,
                }
            }),
        }
    }

    pub async fn get<Res>(&self, path: &str) -> Result<Res, Error>
    where
        C: Codec + Encoder<()> + Decoder<Res>,
        <C as Codec>::Error: From<<C as Encoder<()>>::Error> + From<<C as Decoder<Res>>::Error>,
    {
        self.send::<(), Res>(http::Method::GET, path, None, None)
            .await
    }

    pub async fn get_auth<Res>(&self, credential: &str, path: &str) -> Result<Res, Error>
    where
        C: Codec + Encoder<()> + Decoder<Res>,
        <C as Codec>::Error: From<<C as Encoder<()>>::Error> + From<<C as Decoder<Res>>::Error>,
    {
        self.send::<(), Res>(http::Method::GET, path, Some(credential), None)
            .await
    }

    pub async fn post<Req, Res>(&self, path: &str, body: &Req) -> Result<Res, Error>
    where
        C: Codec + Encoder<Req> + Decoder<Res>,
        <C as Codec>::Error: From<<C as Encoder<Req>>::Error> + From<<C as Decoder<Res>>::Error>,
    {
        self.send::<Req, Res>(http::Method::POST, path, None, Some(body))
            .await
    }

    pub async fn post_auth<Req, Res>(
        &self,
        credential: &str,
        path: &str,
        body: &Req,
    ) -> Result<Res, Error>
    where
        C: Codec + Encoder<Req> + Decoder<Res>,
        <C as Codec>::Error: From<<C as Encoder<Req>>::Error> + From<<C as Decoder<Res>>::Error>,
    {
        self.send::<Req, Res>(http::Method::POST, path, Some(credential), Some(body))
            .await
    }
}

impl<T: Transport, C: Codec> Client<T, C> {
    pub async fn info(&self) -> Result<wire::info::Response, Error> {
        self.get(wire::info::PATH).await
    }

    /// Not authenticated — this is the bootstrapping call that establishes
    /// the credential in the first place.
    pub async fn reg(
        &self,
        body: &wire::reg::cobbl3::Body,
    ) -> Result<wire::reg::cobbl3::Response, Error> {
        self.post(wire::reg::PATH, body).await
    }

    pub async fn squash(
        &self,
        credential: &str,
        body: &wire::auth::squash::Body,
    ) -> Result<wire::auth::squash::Response, Error> {
        self.post_auth(credential, wire::auth::squash::PATH, body)
            .await
    }

    pub async fn state(&self, credential: &str) -> Result<wire::auth::state::Response, Error> {
        self.get_auth(credential, wire::auth::state::PATH).await
    }

    pub async fn pay_bolt11_quote(
        &self,
        credential: &str,
        body: &wire::auth::pay::bolt11::quote::Body,
    ) -> Result<wire::auth::pay::bolt11::quote::Response, Error> {
        self.post_auth(credential, wire::auth::pay::bolt11::quote::PATH, body)
            .await
    }

    pub async fn pay_bolt11_commit(
        &self,
        credential: &str,
        body: &wire::auth::pay::bolt11::commit::Body,
    ) -> Result<wire::auth::pay::bolt11::commit::Response, Error> {
        self.post_auth(credential, wire::auth::pay::bolt11::commit::PATH, body)
            .await
    }
}

impl<T: Transport> Client<T, codec::Any> {
    pub fn from_config(transport: T, config: &Config) -> Self {
        let codec = codec::Any::from(config.codec());
        Self::new(transport, codec, config.base_url())
    }
}
