use crate::{Decoder, Encoder, HeaderPolicy, prelude::*, url};
use crate::{HttpTransport, RequestBuilder};

use core::fmt::Debug;

pub struct Client<T, C> {
    pub(crate) transport: T,
    pub(crate) codec: C,
    pub(crate) base_url: String,
    pub(crate) policies: Vec<Box<dyn HeaderPolicy>>,
}

impl<T: HttpTransport, C> Client<T, C> {
    pub fn new(transport: T, codec: C, base_url: String) -> Self {
        Self {
            transport,
            codec,
            base_url: url::clean_base(&base_url).to_string(),
            policies: Vec::new(),
        }
    }

    pub fn base_url(&self) -> &str {
        self.base_url.as_str()
    }

    pub fn with_policy(mut self, policy: impl HeaderPolicy + 'static) -> Self {
        self.policies.push(Box::new(policy));
        self
    }

    pub fn request(&self, method: http::Method, path: &str) -> RequestBuilder<'_, T, C> {
        RequestBuilder::new(self, method, path)
    }

    // ---- CONVENIENCE METHODS ----------------------------------------

    pub async fn get<Res>(
        &self,
        path: &str,
    ) -> Result<Res, ClientError<T::Error, <C as Encoder<()>>::Error, <C as Decoder<Res>>::Error>>
    where
        C: Encoder<()> + Decoder<Res>,
    {
        self.request(http::Method::GET, path)
            .send::<(), Res>(None)
            .await
    }

    pub async fn get_with_headers<Res>(
        &self,
        path: &str,
        headers: &[(&str, &str)],
    ) -> Result<Res, ClientError<T::Error, <C as Encoder<()>>::Error, <C as Decoder<Res>>::Error>>
    where
        C: Encoder<()> + Decoder<Res>,
    {
        self.request(http::Method::GET, path)
            .map_builder(|mut b| {
                for (k, v) in headers {
                    b = b.header(*k, *v);
                }
                b
            })
            .send::<(), Res>(None)
            .await
    }

    pub async fn post<Req, Res>(
        &self,
        path: &str,
        body: &Req,
    ) -> Result<Res, ClientError<T::Error, <C as Encoder<Req>>::Error, <C as Decoder<Res>>::Error>>
    where
        C: Encoder<Req> + Decoder<Res>,
    {
        self.request(http::Method::POST, path)
            .send::<Req, Res>(Some(body))
            .await
    }

    pub async fn post_with_headers<Req, Res>(
        &self,
        path: &str,
        body: &Req,
        headers: &[(&str, &str)],
    ) -> Result<Res, ClientError<T::Error, <C as Encoder<Req>>::Error, <C as Decoder<Res>>::Error>>
    where
        C: Encoder<Req> + Decoder<Res>,
    {
        self.request(http::Method::POST, path)
            .map_builder(|mut b| {
                for (k, v) in headers {
                    b = b.header(*k, *v);
                }
                b
            })
            .send::<Req, Res>(Some(body))
            .await
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ClientError<TErr: Debug, EncErr: Debug, DecErr: Debug> {
    #[error("Transport error: {0:?}")]
    Transport(TErr),
    #[error("Encode error: {0:?}")]
    Encode(EncErr),
    #[error("Decode error: {0:?}")]
    Decode(DecErr),
    #[error("HTTP construction error: {0}")]
    Http(#[from] http::Error),
    #[error("Server returned status error: {0}")]
    Status(http::StatusCode),
    #[error("Builder was corrupted or already consumed")]
    BuilderCorrupted,
}
