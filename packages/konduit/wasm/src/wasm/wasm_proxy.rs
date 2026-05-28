/// A (sometimes) convenient trait to refer to the proxied type from the proxy in a reflexive
/// manner. Particularly useful when the proxy is generic and not referenced explicitly.
pub trait WasmProxy {
    type T;
}

#[macro_export]
macro_rules! wasm_proxy_min_api {
    ($wrapper:ident => $parent:ty) => {
        impl $crate::wasm::WasmProxy for $wrapper {
            type T = $parent;
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
