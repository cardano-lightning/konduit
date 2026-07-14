mod accept;
pub use accept::*;

mod content_type;
pub use content_type::*;

mod custom;
pub use custom::*;

pub trait HeaderPolicy: Send + Sync {
    fn apply(&self, builder: http::request::Builder, body: Option<&[u8]>)
    -> http::request::Builder;
}
