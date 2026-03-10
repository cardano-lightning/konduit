use crate::{HttpClient, wasm_proxy};
use std::rc::Rc;

wasm_proxy! {
    #[derive(Clone)]
    #[doc = "A reference to a Cardano connector."]
    Connector => Rc<crate::Connector<HttpClient>>
}
