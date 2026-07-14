use crate::{Decoder, HeaderPolicy};

pub struct Accept(pub &'static str);
impl Accept {
    pub fn from_decoder<T>(decoder: &impl Decoder<T>) -> Self {
        Self(decoder.accept_type())
    }

    pub fn boxed(self) -> Box<dyn HeaderPolicy> {
        Box::new(self)
    }
}

impl HeaderPolicy for Accept {
    fn apply(
        &self,
        mut builder: http::request::Builder,
        _body: Option<&[u8]>,
    ) -> http::request::Builder {
        if let Some(headers) = builder.headers_mut() {
            headers.remove(http::header::ACCEPT);
        }
        builder.header(http::header::ACCEPT, self.0)
    }
}
