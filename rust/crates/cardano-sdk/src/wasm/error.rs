//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use wasm_bindgen::prelude::*;

pub type Result<T> = std::result::Result<T, AsJsError>;

#[wasm_bindgen]
#[repr(transparent)]
pub struct AsJsError(JsError);

impl From<anyhow::Error> for AsJsError {
    fn from(e: anyhow::Error) -> Self {
        let e: &(dyn std::error::Error + 'static) = e.as_ref();
        Self(JsError::new(e.to_string().as_str()))
    }
}
