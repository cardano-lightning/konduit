//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::cbor;
use anyhow::anyhow;
use std::fmt;

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

impl fmt::Display for PlutusVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v{}", u8::from(*self))
    }
}

// ----------------------------------------------------------- Converting (from)

impl TryFrom<u8> for PlutusVersion {
    type Error = anyhow::Error;

    fn try_from(version: u8) -> anyhow::Result<Self> {
        match version {
            1 => Ok(PlutusVersion::V1),
            2 => Ok(PlutusVersion::V2),
            3 => Ok(PlutusVersion::V3),
            _ => Err(anyhow!(
                "unknown plutus version version={version}; only 1, 2 and 3 are known"
            )),
        }
    }
}

// ------------------------------------------------------------- Converting (to)

impl From<PlutusVersion> for u8 {
    fn from(version: PlutusVersion) -> Self {
        match version {
            PlutusVersion::V1 => 1,
            PlutusVersion::V2 => 2,
            PlutusVersion::V3 => 3,
        }
    }
}
