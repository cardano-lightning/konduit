// prelude.rs

#[cfg(feature = "std")]
pub use std::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};

#[cfg(not(feature = "std"))]
pub use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};
