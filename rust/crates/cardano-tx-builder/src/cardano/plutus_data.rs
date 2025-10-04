//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::pallas;
use num::ToPrimitive;
use num_bigint::BigInt;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PlutusData(pallas::PlutusData);

// ----------------------------------------------------------- PlutusData

impl PlutusData {
    pub fn integer(i: BigInt) -> Self {
        Self(match i.to_i128().map(pallas::Int::try_from) {
            Some(Ok(i)) => pallas::PlutusData::BigInt(pallas::BigInt::Int(i)),
            _ => {
                let (sign, bytes) = i.to_bytes_be();
                match sign {
                    num_bigint::Sign::Minus => {
                        pallas::PlutusData::BigInt(pallas::BigInt::BigNInt(bytes.into()))
                    }
                    _ => pallas::PlutusData::BigInt(pallas::BigInt::BigUInt(bytes.into())),
                }
            }
        })
    }

    pub fn bytestring(bytes: Vec<u8>) -> Self {
        Self(pallas::PlutusData::BoundedBytes(
            pallas::BoundedBytes::from(bytes),
        ))
    }

    pub fn map(kvs: Vec<(Self, Self)>) -> Self {
        Self(pallas::PlutusData::Map(pallas::KeyValuePairs::from(
            kvs.into_iter()
                .map(|(k, v)| (pallas::PlutusData::from(k), pallas::PlutusData::from(v)))
                .collect::<Vec<_>>(),
        )))
    }

    pub fn list(elems: Vec<Self>) -> Self {
        Self(pallas::PlutusData::Array(if elems.is_empty() {
            pallas::MaybeIndefArray::Def(vec![])
        } else {
            pallas::MaybeIndefArray::Indef(
                elems.into_iter().map(pallas::PlutusData::from).collect(),
            )
        }))
    }

    pub fn constr(ix: u64, fields: Vec<Self>) -> Self {
        let fields = if fields.is_empty() {
            pallas::MaybeIndefArray::Def(vec![])
        } else {
            pallas::MaybeIndefArray::Indef(
                fields.into_iter().map(pallas::PlutusData::from).collect(),
            )
        };

        // NOTE: see https://github.com/input-output-hk/plutus/blob/9538fc9829426b2ecb0628d352e2d7af96ec8204/plutus-core/plutus-core/src/PlutusCore/Data.hs#L139-L155
        Self(if ix < 7 {
            pallas::PlutusData::Constr(pallas::Constr {
                tag: 121 + ix,
                any_constructor: None,
                fields,
            })
        } else if ix < 128 {
            pallas::PlutusData::Constr(pallas::Constr {
                tag: 1280 + ix - 7,
                any_constructor: None,
                fields,
            })
        } else {
            pallas::PlutusData::Constr(pallas::Constr {
                tag: 102,
                any_constructor: Some(ix),
                fields,
            })
        })
    }
}

// ----------------------------------------------------------- Converting (from)

impl From<pallas::PlutusData> for PlutusData {
    fn from(data: pallas::PlutusData) -> Self {
        Self(data)
    }
}

// ------------------------------------------------------------- Converting (to)

impl From<PlutusData> for pallas::PlutusData {
    fn from(data: PlutusData) -> Self {
        data.0
    }
}
