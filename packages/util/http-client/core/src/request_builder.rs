use crate::prelude::*;
use crate::{ClientError, Decoder, Encoder, HttpClient, HttpTransport};

pub struct RequestBuilder<'a, T, C> {
    client: &'a HttpClient<T, C>,
    // Option allows us to `take` in `map_builder` without offending the type or borrow checker.
    builder: Option<http::request::Builder>,
}

pub fn url(base_url: &str, path: &str) -> String {
    let clean_path = path.strip_prefix('/').unwrap_or(path);
    let mut url = String::with_capacity(base_url.len() + 1 + clean_path.len());
    url.push_str(base_url);
    url.push('/');
    url.push_str(clean_path);
    url
}

impl<'a, T: HttpTransport, C> RequestBuilder<'a, T, C> {
    pub fn new(client: &'a HttpClient<T, C>, method: http::Method, path: &str) -> Self {
        let builder = Some(
            http::Request::builder()
                .method(method)
                .uri(url(&client.base_url, path)),
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
        mut self,
        body: Option<&Req>,
    ) -> Result<Res, ClientError<T::Error, <C as Encoder<Req>>::Error, <C as Decoder<Res>>::Error>>
    where
        C: Encoder<Req> + Decoder<Res>,
    {
        let mut native_builder = self.builder.take().ok_or(ClientError::BuilderCorrupted)?;

        let payload = match body {
            Some(b) => {
                native_builder = native_builder
                    .header(http::header::CONTENT_TYPE, self.client.codec.content_type());
                self.client.codec.encode(b).map_err(ClientError::Encode)?
            }
            None => Vec::new(),
        };

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
        let res_body = self
            .client
            .codec
            .decode(response.body())
            .map_err(ClientError::Decode)?;
        Ok(res_body)
    }
}
