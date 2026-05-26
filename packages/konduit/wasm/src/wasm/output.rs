use crate::{core, wasm_proxy};

wasm_proxy! {
    #[derive(Debug, Clone)]
    #[doc = "A transaction output, which comprises of at least an Address and a Value."]
    Output => core::Output
}
