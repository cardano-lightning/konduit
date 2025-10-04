//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{cbor, pallas};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, cbor::Encode, cbor::Decode)]
#[repr(transparent)]
#[cbor(transparent)]
pub struct RedeemerPointer(#[n(0)] pallas::RedeemersKey);

// --------------------------------------------------------------------- Building

impl RedeemerPointer {
    pub fn mint(index: u32) -> Self {
        RedeemerPointer(pallas::RedeemersKey {
            tag: pallas::RedeemerTag::Mint,
            index,
        })
    }

    pub fn spend(index: u32) -> Self {
        RedeemerPointer(pallas::RedeemersKey {
            tag: pallas::RedeemerTag::Spend,
            index,
        })
    }
}

// ------------------------------------------------------------ Converting (from)

impl From<pallas::RedeemersKey> for RedeemerPointer {
    fn from(key: pallas::RedeemersKey) -> Self {
        Self(key)
    }
}

// -------------------------------------------------------------- Converting (to)

impl From<RedeemerPointer> for pallas::RedeemersKey {
    fn from(ptr: RedeemerPointer) -> Self {
        ptr.0
    }
}
