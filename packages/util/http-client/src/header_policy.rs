use crate::{Decoder, Encoder};

pub trait HeaderPolicy: Send + Sync {
    fn apply(&self, builder: http::request::Builder, body: Option<&[u8]>)
    -> http::request::Builder;
}

pub struct Accept(pub &'static str);
impl Accept {
    pub fn from_decoder(decoder: &impl Decoder<()>) -> Self {
        Self(decoder.accept_type())
    }
}

impl HeaderPolicy for Accept {
    fn apply(
        &self,
        builder: http::request::Builder,
        _body: Option<&[u8]>,
    ) -> http::request::Builder {
        builder.header(http::header::ACCEPT, self.0)
    }
}

pub struct ContentType(pub &'static str);
impl ContentType {
    pub fn from_encoder<T>(encoder: &impl Encoder<T>) -> Self {
        Self(encoder.content_type())
    }
}

impl HeaderPolicy for ContentType {
    fn apply(
        &self,
        builder: http::request::Builder,
        body: Option<&[u8]>,
    ) -> http::request::Builder {
        match body {
            Some(_) => builder.header(http::header::CONTENT_TYPE, self.0),
            None => builder,
        }
    }
}

pub struct ContentLength;
impl HeaderPolicy for ContentLength {
    fn apply(
        &self,
        builder: http::request::Builder,
        body: Option<&[u8]>,
    ) -> http::request::Builder {
        match body {
            Some(bytes) => builder.header(http::header::CONTENT_LENGTH, bytes.len()),
            None => builder,
        }
    }
}

pub struct ContentNegotiation(pub &'static str);

impl ContentNegotiation {
    pub fn from_encoder(encoder: &impl Encoder<()>) -> Self {
        Self(encoder.content_type())
    }
}

impl HeaderPolicy for ContentNegotiation {
    fn apply(
        &self,
        builder: http::request::Builder,
        body: Option<&[u8]>,
    ) -> http::request::Builder {
        match body {
            Some(_) => builder.header(http::header::CONTENT_TYPE, self.0),
            None => builder.header(http::header::ACCEPT, self.0),
        }
    }
}
