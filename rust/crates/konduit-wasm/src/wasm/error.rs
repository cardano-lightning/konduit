use std::rc::Rc;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[repr(transparent)]
/// @hidden
pub struct Error(Rc<Box<dyn std::error::Error + Send + Sync + 'static>>);

#[wasm_bindgen]
impl Error {
    #[wasm_bindgen(getter, js_name = "message")]
    pub fn _wasm_message(&self) -> String {
        format!("{}", self.0)
    }
}

impl From<anyhow::Error> for Error {
    fn from(e: anyhow::Error) -> Self {
        Self(Rc::new(e.into_boxed_dyn_error()))
    }
}

impl From<&Error> for Error {
    fn from(e: &Error) -> Self {
        Self(e.0.clone())
    }
}

impl From<&mut Error> for Error {
    fn from(e: &mut Error) -> Self {
        Self(e.0.clone())
    }
}
