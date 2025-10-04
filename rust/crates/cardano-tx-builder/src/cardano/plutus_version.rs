//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::cbor;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, cbor::Encode, cbor::Decode)]
#[cbor(index_only)]
pub enum PlutusVersion {
    #[n(0)]
    V1,
    #[n(1)]
    V2,
    #[n(2)]
    V3,
}

// -------------------------------------------------------------- Converting (to)

impl From<PlutusVersion> for u8 {
    fn from(version: PlutusVersion) -> Self {
        match version {
            PlutusVersion::V1 => 1,
            PlutusVersion::V2 => 2,
            PlutusVersion::V3 => 3,
        }
    }
}
