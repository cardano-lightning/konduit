//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

#![cfg(feature = "wasm")]

mod error;
pub use error::*;

#[macro_export]
macro_rules! wasm_proxy_min_api {
    ($wrapper:ident) => {
        impl ::std::ops::Deref for $wrapper {
            type Target = super::$wrapper;

            #[inline]
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl ::std::convert::From<$wrapper> for super::$wrapper {
            #[inline]
            fn from(w: $wrapper) -> Self {
                w.0
            }
        }

        impl ::std::convert::From<super::$wrapper> for $wrapper {
            #[inline]
            fn from(v: super::$wrapper) -> Self {
                Self(v)
            }
        }
    };
}

#[macro_export]
macro_rules! wasm_proxy {
    (
        $(#[$attr:meta])*
        $name:ident
    ) => {
        #[wasm_bindgen::prelude::wasm_bindgen]
        #[repr(transparent)]
        $(#[$attr])*
        pub struct $name(super::$name);

        $crate::wasm_proxy_min_api!($name);
    };
}
