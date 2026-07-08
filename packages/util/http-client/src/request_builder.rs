use crate::{Client, Decoder, Encoder, Transport, client};
use crate::{HeaderPolicy, prelude::*, url};

/// Collapses the three-way `client::Error` generic so call sites don't have
/// to spell out `T::Error` / `Encoder`/`Decoder` associated errors each time.
pub type SendResult<T, C, Req, Res> = Result<
    Res,
    client::Error<<T as Transport>::Error, <C as Encoder<Req>>::Error, <C as Decoder<Res>>::Error>,
>;

pub struct RequestBuilder<'a, T, C> {
    client: &'a Client<T, C>,
    builder: http::request::Builder,
}

impl<'a, T: Transport, C> RequestBuilder<'a, T, C> {
    pub fn new(client: &'a Client<T, C>, method: http::Method, path: &str) -> Self {
        let builder = http::Request::builder()
            .method(method)
            .uri(url::clean_join(&client.base_url, path));
        Self { client, builder }
    }

    /// The hatch to the underlying http RequestBuilder.
    /// The exception: use `send` with `body` to use the `Encoder`.
    pub fn map_builder<F>(mut self, f: F) -> Self
    where
        F: FnOnce(http::request::Builder) -> http::request::Builder,
    {
        self.builder = f(self.builder);
        self
    }

    pub async fn send<Req, Res>(self, body: Option<&Req>) -> SendResult<T, C, Req, Res>
    where
        C: Encoder<Req> + Decoder<Res>,
    {
        let policies = &self.client.policies;
        self.send_with_policies(body, policies).await
    }

    pub async fn send_no_policy<Req, Res>(self, body: Option<&Req>) -> SendResult<T, C, Req, Res>
    where
        C: Encoder<Req> + Decoder<Res>,
    {
        self.send_with_policies(body, &[]).await
    }

    async fn send_with_policies<Req, Res>(
        self,
        body: Option<&Req>,
        policies: &[Box<dyn HeaderPolicy>],
    ) -> SendResult<T, C, Req, Res>
    where
        C: Encoder<Req> + Decoder<Res>,
    {
        let mut builder = self.builder;
        let payload = match body {
            Some(b) => self.client.codec.encode(b).map_err(client::Error::Encode)?,
            None => Vec::new(),
        };
        let body_slice = (!payload.is_empty()).then(|| payload.as_slice());

        for policy in policies {
            builder = policy.apply(builder, body_slice);
        }

        let request = builder.body(payload)?;
        let response = self
            .client
            .transport
            .transport(request)
            .await
            .map_err(client::Error::Transport)?;

        if !response.status().is_success() {
            return Err(client::Error::Response {
                status: response.status(),
                body: response.body().clone(),
            });
        }

        self.client
            .codec
            .decode(response.body())
            .map_err(client::Error::Decode)
    }
}
