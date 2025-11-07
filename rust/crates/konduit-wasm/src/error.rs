use wasm_bindgen::prelude::*;

pub type Result<T> = std::result::Result<T, StrError>;

#[wasm_bindgen]
#[repr(transparent)]
pub struct StrError(String);

#[wasm_bindgen]
impl StrError {
    #[wasm_bindgen(js_name = "toString")]
    pub fn wasm_to_string(&self) -> String {
        self.0.clone()
    }
}

impl From<anyhow::Error> for StrError {
    fn from(e: anyhow::Error) -> Self {
        let e: &(dyn std::error::Error + 'static) = e.as_ref();
        Self(e.to_string())
    }
}
