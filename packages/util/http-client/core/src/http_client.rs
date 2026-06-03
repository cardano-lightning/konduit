use std::future::Future;

pub use http::Method;

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Minimal HTTP Client. One method, bytes in, bytes out.
///
/// Two implementations: reqwest (native) and gloo_net (wasm).
/// Everything above this — headers policy, content negotiation,
/// typed deserialization — belongs downstream
pub trait HttpClient {
    type Error;

    fn base_url(&self) -> &str;

    fn request(
        &self,
        method: &Method,
        path: &str,
        headers: &[(&str, &str)],
        body: Option<&[u8]>,
    ) -> impl Future<Output = Result<Vec<u8>, Self::Error>>;

    fn get(
        &self,
        path: &str,
        headers: &[(&str, &str)],
    ) -> impl Future<Output = Result<Vec<u8>, Self::Error>> {
        self.request(&Method::GET, path, headers, None)
    }

    fn post(
        &self,
        path: &str,
        headers: &[(&str, &str)],
        body: &[u8],
    ) -> impl Future<Output = Result<Vec<u8>, Self::Error>> {
        self.request(&Method::POST, path, headers, Some(body))
    }
}
