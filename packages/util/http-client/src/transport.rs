/// Minimal HTTP Transport. One method, bytes in, bytes out, no state.
///
/// Two implementations: reqwest (native) and gloo_net (wasm).
/// Everything above this - headers policy, content negotiation,
/// typed deserialization - belongs downstream
pub trait Transport: Send + Sync {
    type Error: core::error::Error + Send + Sync + 'static;

    fn transport(
        &self,
        req: http::Request<Vec<u8>>,
    ) -> impl core::future::Future<Output = Result<http::Response<Vec<u8>>, Self::Error>> + Send;
}

#[cfg(feature = "gloo")]
mod gloo;
#[cfg(feature = "gloo")]
pub use gloo::{GlooTransport as Gloo, GlooTransportError as GlooError};

#[cfg(feature = "reqwest")]
mod reqwest;
#[cfg(feature = "reqwest")]
pub use reqwest::{ReqwestTransport as Reqwest, ReqwestTransportError as ReqwestError};
