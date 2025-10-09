//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{cbor, pallas};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, cbor::Encode, cbor::Decode)]
#[repr(transparent)]
#[cbor(transparent)]
pub struct RedeemerPointer(#[n(0)] pallas::RedeemersKey);

impl fmt::Display for RedeemerPointer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0.tag {
            pallas_primitives::conway::RedeemerTag::Mint => write!(f, "Mint({})", self.0.index),
            pallas_primitives::conway::RedeemerTag::Spend => write!(f, "Spend({})", self.0.index),
            pallas_primitives::conway::RedeemerTag::Reward => {
                write!(f, "Withdraw({})", self.0.index)
            }
            pallas_primitives::conway::RedeemerTag::Cert => write!(f, "Publish({})", self.0.index),
            pallas_primitives::conway::RedeemerTag::Vote => write!(f, "Vote({})", self.0.index),
            pallas_primitives::conway::RedeemerTag::Propose => {
                write!(f, "Propose({})", self.0.index)
            }
        }
    }
}

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
