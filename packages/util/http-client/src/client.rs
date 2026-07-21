use crate::{Decoder, Encoder, HeaderPolicy, Transport, header_policy, url};

use core::fmt::Debug;

#[cfg(feature = "gloo")]
mod gloo;
#[cfg(feature = "gloo")]
pub use gloo::GlooClient as Gloo;

pub struct Client<T, C> {
    pub(crate) transport: T,
    pub(crate) codec: C,
    pub(crate) base_url: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ClientError<Transport, Encode, Decode> {
    #[error("Transport error")]
    Transport(#[source] Transport),
    #[error("Encode error")]
    Encode(#[source] Encode),
    #[error("Decode error")]
    Decode(#[source] Decode),
    #[error("HTTP error")]
    Http(#[source] http::Error),
    #[error("Server returned status error: {0}")]
    Status(http::StatusCode),
    #[error("Builder was corrupted or already consumed")]
    BuilderCorrupted,
}

impl<T: Transport, C> Client<T, C> {
    pub fn new(transport: T, codec: C, base_url: String) -> Self {
        Self {
            transport,
            base_url: url::clean_base(&base_url).to_string(),
            codec,
        }
    }

    pub fn base_url(&self) -> &str {
        self.base_url.as_str()
    }

    // ---- CONVENIENCE METHODS ----------------------------------------

    pub async fn get<Res>(
        &self,
        path: &str,
    ) -> Result<Res, ClientError<T::Error, <C as Encoder<()>>::Error, <C as Decoder<Res>>::Error>>
    where
        C: Encoder<()> + Decoder<Res>,
    {
        self.get_with_headers(path, vec![]).await
    }

    pub async fn get_with_headers<Res>(
        &self,
        path: &str,
        custom_headers: Vec<Box<dyn HeaderPolicy>>,
    ) -> Result<Res, ClientError<T::Error, <C as Encoder<()>>::Error, <C as Decoder<Res>>::Error>>
    where
        C: Encoder<()> + Decoder<Res>,
    {
        let mut policies = vec![header_policy::Accept::from_decoder(&self.codec).boxed()];
        policies.extend(custom_headers);
        self.send::<(), Res>(None, self.request(http::Method::GET, path), &policies)
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
        self.post_with_headers(path, body, vec![]).await
    }

    pub async fn post_with_headers<Req, Res>(
        &self,
        path: &str,
        body: &Req,
        custom_headers: Vec<Box<dyn HeaderPolicy>>,
    ) -> Result<Res, ClientError<T::Error, <C as Encoder<Req>>::Error, <C as Decoder<Res>>::Error>>
    where
        C: Encoder<Req> + Decoder<Res>,
    {
        let mut policies = vec![
            Box::new(header_policy::ContentType::from_encoder(&self.codec)).boxed(),
            Box::new(header_policy::Accept::from_decoder(&self.codec)).boxed(),
        ];
        policies.extend(custom_headers);

        self.send::<Req, Res>(
            Some(body),
            self.request(http::Method::POST, path),
            &policies,
        )
        .await
    }

    // ---- REQUEST BUILDER ----------------------------------------------------

    fn request(&self, method: http::Method, path: &str) -> http::request::Builder {
        http::Request::builder()
            .method(method)
            .uri(url::clean_join(&self.base_url, path))
    }

    async fn send<Req, Res>(
        &self,
        body: Option<&Req>,
        mut builder: http::request::Builder,
        policies: &[Box<dyn HeaderPolicy>],
    ) -> Result<Res, ClientError<T::Error, <C as Encoder<Req>>::Error, <C as Decoder<Res>>::Error>>
    where
        C: Encoder<Req> + Decoder<Res>,
    {
        let payload = match body {
            Some(b) => self.codec.encode(b).map_err(ClientError::Encode)?,
            None => Vec::new(),
        };

        let body_slice = if payload.is_empty() {
            None
        } else {
            Some(payload.as_slice())
        };

        for policy in policies {
            builder = policy.apply(builder, body_slice);
        }

        let request = builder.body(payload).map_err(ClientError::Http)?;

        let response = self
            .transport
            .transport(request)
            .await
            .map_err(ClientError::Transport)?;

        if !response.status().is_success() {
            return Err(ClientError::Status(response.status()));
        }

        self.codec
            .decode(response.body())
            .map_err(ClientError::Decode)
    }
}
