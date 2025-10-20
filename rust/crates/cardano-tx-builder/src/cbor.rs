//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

pub use minicbor::{decode::Decode, encode::Encode};
pub use pallas_codec::minicbor::*;
use std::convert::Infallible;

/// A trait mostly for convenience, as we often end up writing bytes to CBOR. The original
/// [`minicbor::Encode::encode`] makes room for encoding into any Writer type, and thus provides
/// the ability to fail.
///
/// When writing bytes to a vector, the operation is however Infaillible.
pub trait ToCbor {
    fn to_cbor(&self) -> Vec<u8>;
}

impl<T: Encode<()>> ToCbor for T {
    fn to_cbor(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        let _: Result<(), encode::Error<Infallible>> = encode(self, &mut bytes);
        bytes
    }
}
