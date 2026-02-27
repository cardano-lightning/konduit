pub trait HttpClient {
    type Error;

    fn to_json<T: serde::Serialize>(value: &T) -> Vec<u8>;

    fn get<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
    ) -> impl Future<Output = Result<T, Self::Error>> {
        self.get_with_headers(path, &[])
    }

    fn get_with_headers<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        headers: &[(&str, &str)],
    ) -> impl Future<Output = Result<T, Self::Error>>;

    fn post<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: impl AsRef<[u8]>,
    ) -> impl Future<Output = Result<T, Self::Error>> {
        self.post_with_headers(path, &[], body)
    }

    fn post_with_headers<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        headers: &[(&str, &str)],
        body: impl AsRef<[u8]>,
    ) -> impl Future<Output = Result<T, Self::Error>>;
}
