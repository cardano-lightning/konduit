// prelude.rs

#[cfg(feature = "std")]
pub use std::{
    string::{String, ToString},
    vec::Vec,
};

#[cfg(not(feature = "std"))]
pub use alloc::{
    string::{String, ToString},
    vec::Vec,
};
