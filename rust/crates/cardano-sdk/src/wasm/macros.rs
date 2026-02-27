//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

pub trait WasmProxy {
    type OriginalType;
}

#[macro_export]
macro_rules! wasm_proxy_min_api {
    ($wrapper:ident => $parent:ty) => {
        impl $crate::wasm::WasmProxy for $wrapper {
            type OriginalType = $parent;
        }

        impl ::std::ops::Deref for $wrapper {
            type Target = $parent;

            #[inline]
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl ::std::borrow::Borrow<$parent> for $wrapper {
            #[inline]
            fn borrow(&self) -> &$parent {
                &self.0
            }
        }

        impl ::std::convert::From<$wrapper> for $parent {
            #[inline]
            fn from(w: $wrapper) -> Self {
                w.0
            }
        }

        impl ::std::convert::From<$parent> for $wrapper {
            #[inline]
            fn from(v: $parent) -> Self {
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
        $(#[$attr])*
        #[wasm_bindgen::prelude::wasm_bindgen]
        #[repr(transparent)]
        pub struct $name(super::$name);

        $crate::wasm_proxy_min_api!($name => super::$name);
    };

    (
        $(#[$attr:meta])*
        $name:ident => $parent:ty
    ) => {
        $(#[$attr])*
        #[wasm_bindgen::prelude::wasm_bindgen]
        #[repr(transparent)]
        pub struct $name($parent);

        $crate::wasm_proxy_min_api!($name => $parent);
    };
}
