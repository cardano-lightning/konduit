#![doc = include_str!("../README.md")]

mod wire;
pub use wire::*;

#[cfg(feature = "actix")]
mod actix;
#[cfg(feature = "actix")]
pub use actix::*;

#[doc(hidden)]
pub extern crate self as problem_details;
