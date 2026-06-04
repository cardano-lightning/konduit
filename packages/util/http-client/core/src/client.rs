use crate::prelude::*;
use crate::{HttpTransport, RequestBuilder};

use core::fmt::Debug;

pub struct Client<T, C> {
    pub(crate) transport: T,
    pub(crate) codec: C,
    pub(crate) base_url: String,
}

impl<T: HttpTransport, C> Client<T, C> {
    pub fn new(transport: T, codec: C, base_url: String) -> Self {
        Self {
            transport,
            codec,
            base_url: base_url.strip_suffix('/').unwrap_or(&base_url).to_string(),
        }
    }

    pub fn request(&self, method: http::Method, path: &str) -> RequestBuilder<'_, T, C> {
        RequestBuilder::new(self, method, path)
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
