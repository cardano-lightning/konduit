//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{Hash, PlutusVersion, pallas};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PlutusScript(PlutusVersion, Vec<u8>);

// --------------------------------------------------------------------- Building

impl PlutusScript {
    /// Instance a script from its language and serialised form (CBOR + Flat encoding).
    pub fn new(version: PlutusVersion, script: Vec<u8>) -> Self {
        Self(version, script)
    }

    pub fn version(&self) -> PlutusVersion {
        self.0
    }

    pub fn size(&self) -> u64 {
        self.1.len() as u64
    }
}

// ------------------------------------------------------------ Converting (from)

impl From<pallas::PlutusScript<1>> for PlutusScript {
    fn from(plutus_script: pallas::PlutusScript<1>) -> Self {
        Self(PlutusVersion::V1, plutus_script.0.to_vec())
    }
}

impl From<pallas::PlutusScript<2>> for PlutusScript {
    fn from(plutus_script: pallas::PlutusScript<2>) -> Self {
        Self(PlutusVersion::V2, plutus_script.0.to_vec())
    }
}

impl From<pallas::PlutusScript<3>> for PlutusScript {
    fn from(plutus_script: pallas::PlutusScript<3>) -> Self {
        Self(PlutusVersion::V3, plutus_script.0.to_vec())
    }
}

// -------------------------------------------------------------- Converting (to)

impl From<PlutusScript> for Hash<28> {
    fn from(PlutusScript(version, script): PlutusScript) -> Self {
        let mut buffer: Vec<u8> = vec![u8::from(version)];
        buffer.extend_from_slice(script.as_slice());
        Hash::from(pallas::hash::Hasher::<224>::hash(&buffer))
    }
}

pub struct PlutusVersionMismatch {
    pub expected: PlutusVersion,
    pub found: PlutusVersion,
}

impl From<PlutusScript> for pallas::ScriptRef {
    fn from(PlutusScript(version, script): PlutusScript) -> Self {
        match version {
            PlutusVersion::V1 => pallas::ScriptRef::PlutusV1Script(pallas::PlutusScript::<1>(
                pallas::Bytes::from(script),
            )),
            PlutusVersion::V2 => pallas::ScriptRef::PlutusV2Script(pallas::PlutusScript::<2>(
                pallas::Bytes::from(script),
            )),
            PlutusVersion::V3 => pallas::ScriptRef::PlutusV3Script(pallas::PlutusScript::<3>(
                pallas::Bytes::from(script),
            )),
        }
    }
}

impl TryFrom<PlutusScript> for pallas::PlutusScript<1> {
    type Error = PlutusVersionMismatch;

    fn try_from(PlutusScript(version, script): PlutusScript) -> Result<Self, Self::Error> {
        match version {
            PlutusVersion::V1 => Ok(pallas::PlutusScript(pallas::Bytes::from(script))),
            PlutusVersion::V2 | PlutusVersion::V3 => Err(PlutusVersionMismatch {
                expected: PlutusVersion::V1,
                found: version,
            }),
        }
    }
}

impl TryFrom<PlutusScript> for pallas::PlutusScript<2> {
    type Error = PlutusVersionMismatch;

    fn try_from(PlutusScript(version, script): PlutusScript) -> Result<Self, Self::Error> {
        match version {
            PlutusVersion::V2 => Ok(pallas::PlutusScript(pallas::Bytes::from(script))),
            PlutusVersion::V1 | PlutusVersion::V3 => Err(PlutusVersionMismatch {
                expected: PlutusVersion::V2,
                found: version,
            }),
        }
    }
}

impl TryFrom<PlutusScript> for pallas::PlutusScript<3> {
    type Error = PlutusVersionMismatch;

    fn try_from(PlutusScript(version, script): PlutusScript) -> Result<Self, Self::Error> {
        match version {
            PlutusVersion::V3 => Ok(pallas::PlutusScript(pallas::Bytes::from(script))),
            PlutusVersion::V1 | PlutusVersion::V2 => Err(PlutusVersionMismatch {
                expected: PlutusVersion::V3,
                found: version,
            }),
        }
    }
}
