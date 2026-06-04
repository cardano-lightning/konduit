use crate::{ClientError, Decoder, Encoder, HttpClient, HttpTransport};
use crate::{HeaderPolicy, prelude::*, url};

pub struct RequestBuilder<'a, T, C> {
    client: &'a HttpClient<T, C>,
    // Option allows us to `take` in `map_builder` without offending the type or borrow checker.
    builder: Option<http::request::Builder>,
}

impl<'a, T: HttpTransport, C> RequestBuilder<'a, T, C> {
    pub fn new(client: &'a HttpClient<T, C>, method: http::Method, path: &str) -> Self {
        let builder = Some(
            http::Request::builder()
                .method(method)
                .uri(url::clean_join(&client.base_url, path)),
        );
        Self { client, builder }
    }

    /// The hatch to the underlying http RequestBuilder.
    /// The exception: use `send` with `body` to use the `Encoder`.
    pub fn map_builder<F>(mut self, f: F) -> Self
    where
        F: FnOnce(http::request::Builder) -> http::request::Builder,
    {
        if let Some(b) = self.builder.take() {
            self.builder = Some(f(b));
        }
        self
    }

    pub async fn send<Req, Res>(
        self,
        body: Option<&Req>,
    ) -> Result<Res, ClientError<T::Error, <C as Encoder<Req>>::Error, <C as Decoder<Res>>::Error>>
    where
        C: Encoder<Req> + Decoder<Res>,
    {
        let policies = &self.client.policies;
        self.send_with_policies(body, policies).await
    }

    pub async fn send_no_policy<Req, Res>(
        self,
        body: Option<&Req>,
    ) -> Result<Res, ClientError<T::Error, <C as Encoder<Req>>::Error, <C as Decoder<Res>>::Error>>
    where
        C: Encoder<Req> + Decoder<Res>,
    {
        self.send_with_policies(body, &[]).await
    }

    async fn send_with_policies<Req, Res>(
        mut self,
        body: Option<&Req>,
        policies: &[Box<dyn HeaderPolicy>],
    ) -> Result<Res, ClientError<T::Error, <C as Encoder<Req>>::Error, <C as Decoder<Res>>::Error>>
    where
        C: Encoder<Req> + Decoder<Res>,
    {
        let mut native_builder = self.builder.take().ok_or(ClientError::BuilderCorrupted)?;

        let payload = match body {
            Some(b) => self.client.codec.encode(b).map_err(ClientError::Encode)?,
            None => Vec::new(),
        };

        let body_slice = if payload.is_empty() {
            None
        } else {
            Some(payload.as_slice())
        };

        for policy in policies {
            native_builder = policy.apply(native_builder, body_slice);
        }

        let request = native_builder.body(payload)?;
        let response = self
            .client
            .transport
            .transport(request)
            .await
            .map_err(ClientError::Transport)?;

        if !response.status().is_success() {
            return Err(ClientError::Status(response.status()));
        }

        self.client
            .codec
            .decode(response.body())
            .map_err(ClientError::Decode)
    }
}
