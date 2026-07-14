use crate::{Encoder, HeaderPolicy};

pub struct ContentType(pub &'static str);
impl ContentType {
    pub fn from_encoder<T>(encoder: &impl Encoder<T>) -> Self {
        Self(encoder.content_type())
    }

    pub fn boxed(self) -> Box<dyn HeaderPolicy> {
        Box::new(self)
    }
}

impl HeaderPolicy for ContentType {
    fn apply(
        &self,
        mut builder: http::request::Builder,
        body: Option<&[u8]>,
    ) -> http::request::Builder {
        if let Some(headers) = builder.headers_mut() {
            headers.remove(http::header::CONTENT_TYPE);
        }
        match body {
            Some(_) => builder.header(http::header::CONTENT_TYPE, self.0),
            None => builder,
        }
    }
}
