//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use wasm_bindgen::prelude::*;

pub type Result<T> = std::result::Result<T, AsJsError>;

#[wasm_bindgen]
#[repr(transparent)]
/// @hidden
pub struct AsJsError(anyhow::Error);

#[wasm_bindgen]
impl AsJsError {
    #[wasm_bindgen(getter, js_name = "message")]
    pub fn _wasm_message(&self) -> String {
        format!("{}", self.0)
    }
}

impl From<anyhow::Error> for AsJsError {
    fn from(e: anyhow::Error) -> Self {
        Self(e)
    }
}
