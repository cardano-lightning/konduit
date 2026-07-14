use crate::HeaderPolicy;
use http::header::HeaderValue;

pub struct Custom(&'static str, HeaderValue);
impl Custom {
    pub fn new(key: &'static str, value: &str) -> Self {
        Self(
            key,
            HeaderValue::try_from(value).unwrap_or_else(|e| {
                panic!("invalid '{key}' header value '{value}' provided to Custom policy: {e}")
            }),
        )
    }

    pub fn boxed(self) -> Box<dyn HeaderPolicy> {
        Box::new(self)
    }
}

impl HeaderPolicy for Custom {
    fn apply(
        &self,
        mut builder: http::request::Builder,
        _body: Option<&[u8]>,
    ) -> http::request::Builder {
        if let Some(headers) = builder.headers_mut() {
            headers.remove(self.0);
        }
        builder.header(self.0, &self.1)
    }
}
