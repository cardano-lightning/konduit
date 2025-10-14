//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{cbor, cbor::ToCbor, pallas};
use num::ToPrimitive;
use num_bigint::BigInt;
use std::fmt;

/// An arbitrary data format used by Plutus smart contracts.
///
/// It can be constructed directly using one of the two variants:
///
/// - [`Self::integer`]
/// - [`Self::bytes`]
///
/// And combine to form larger objects using:
///
/// - [`Self::list`]
/// - [`Self::map`]
/// - [`Self::constr`]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, cbor::Encode, cbor::Decode)]
#[repr(transparent)]
#[cbor(transparent)]
pub struct PlutusData(#[n(0)] pallas::PlutusData);

impl fmt::Display for PlutusData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CBOR({})", hex::encode(self.to_cbor()))
    }
}

// -------------------------------------------------------------------- Building

impl PlutusData {
    /// Construct a data value from an arbitrarily-sized integer.
    ///
    /// # examples
    ///
    /// ```rust
    /// # use cardano_tx_builder::PlutusData;
    /// # use num_bigint::BigInt;
    /// assert_eq!(
    ///     format!("{}", PlutusData::integer(42)),
    ///     "CBOR(182a)",
    /// );
    ///
    /// assert_eq!(
    ///     format!("{}", PlutusData::integer(-14)),
    ///     "CBOR(2d)",
    /// );
    ///
    /// assert_eq!(
    ///     format!("{}", PlutusData::integer(BigInt::from(u128::MAX) + BigInt::from(u128::MAX))),
    ///     "CBOR(c25101fffffffffffffffffffffffffffffffe)",
    /// );
    /// ```
    pub fn integer(i: impl Into<BigInt>) -> Self {
        let i: BigInt = i.into();

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

    /// Construct an arbitrarily-sized byte-array value.
    ///
    /// # examples
    ///
    /// ```rust
    /// # use cardano_tx_builder::PlutusData;
    /// assert_eq!(
    ///     format!("{}", PlutusData::bytes(b"foo")),
    ///     "CBOR(43666f6f)"
    /// );
    ///
    /// assert_eq!(
    ///     format!(
    ///         "{}",
    ///         PlutusData::bytes(
    ///             b"Rerum deleniti nisi ea exercitationem architecto. Quia architecto voluptates error."
    ///         )
    ///     ),
    ///     "CBOR(5f5840526572756d2064656c656e697469206e69736920656120657865726369746174696f6e656d206172636869746563746f2e205175696120617263686974656374536f20766f6c75707461746573206572726f722eff)"
    /// );
    /// ```
    pub fn bytes(bytes: impl AsRef<[u8]>) -> Self {
        Self(pallas::PlutusData::BoundedBytes(
            pallas::BoundedBytes::from(bytes.as_ref().to_vec()),
        ))
    }

    /// Construct an arbitrarily-sized list of [`self::PlutusData`] values.
    ///
    /// # examples
    ///
    /// ```rust
    /// # use cardano_tx_builder::PlutusData;
    ///
    /// assert_eq!(
    ///     format!("{}", PlutusData::list([])),
    ///     "CBOR(80)"
    /// );
    ///
    /// assert_eq!(
    ///     format!("{}", PlutusData::list([
    ///         PlutusData::bytes(b"foo"),
    ///         PlutusData::bytes(b"bar"),
    ///     ])),
    ///     "CBOR(9f43666f6f43626172ff)"
    /// );
    ///
    /// assert_eq!(
    ///     format!("{}", PlutusData::list([
    ///         PlutusData::bytes(b"foo"),
    ///         PlutusData::list([
    ///             PlutusData::integer(1),
    ///             PlutusData::integer(2),
    ///         ]),
    ///     ])),
    ///     "CBOR(9f43666f6f9f0102ffff)"
    /// );
    /// ```
    pub fn list(elems: impl IntoIterator<Item = Self>) -> Self {
        let elems = elems
            .into_iter()
            .map(pallas::PlutusData::from)
            .collect::<Vec<_>>();

        Self(pallas::PlutusData::Array(if elems.is_empty() {
            pallas::MaybeIndefArray::Def(elems)
        } else {
            pallas::MaybeIndefArray::Indef(elems)
        }))
    }

    /// Construct an arbitrarily-sized list of [`self::PlutusData`] values.
    ///
    /// # examples
    ///
    /// ```rust
    /// # use cardano_tx_builder::PlutusData;
    ///
    /// assert_eq!(
    ///     format!("{}", PlutusData::map([])),
    ///     "CBOR(a0)"
    /// );
    ///
    /// assert_eq!(
    ///     format!(
    ///         "{}",
    ///         PlutusData::map([
    ///             (PlutusData::bytes(b"FOO"), PlutusData::integer(1)),
    ///             (PlutusData::bytes(b"BAR"), PlutusData::integer(2)),
    ///         ]),
    ///     ),
    ///     "CBOR(a243464f4f014342415202)"
    /// );
    pub fn map(kvs: impl IntoIterator<Item = (Self, Self)>) -> Self {
        let kvs = kvs
            .into_iter()
            .map(|(k, v)| (pallas::PlutusData::from(k), pallas::PlutusData::from(v)))
            .collect::<Vec<_>>();

        Self(pallas::PlutusData::Map(pallas::KeyValuePairs::from(kvs)))
    }

    /// Construct a tagged variant with [`self::PlutusData`] fields.
    ///
    /// <div class="warning">The constructor index `ix` will typically starts at `0`, and be
    /// encoded accordingly. You may sometimes see libraries or tools working off encoded indexes
    /// (e.g. starting at `121`). This is not the case here.</div>
    ///
    /// # examples
    ///
    /// ```rust
    /// # use cardano_tx_builder::PlutusData;
    ///
    /// assert_eq!(
    ///     format!("{}", PlutusData::constr(0, [])),
    ///     "CBOR(d87980)"
    /// );
    ///
    /// assert_eq!(
    ///     format!(
    ///         "{}",
    ///         PlutusData::constr(0, [
    ///             PlutusData::constr(1, []),
    ///             PlutusData::integer(1337),
    ///         ]),
    ///     ),
    ///     "CBOR(d8799fd87a80190539ff)"
    /// );
    /// ```
    pub fn constr(ix: u64, fields: impl IntoIterator<Item = Self>) -> Self {
        let fields = fields
            .into_iter()
            .map(pallas::PlutusData::from)
            .collect::<Vec<_>>();

        let fields = if fields.is_empty() {
            pallas::MaybeIndefArray::Def(fields)
        } else {
            pallas::MaybeIndefArray::Indef(fields)
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

// ------------------------------------------------------------------ Inspecting

impl PlutusData {
    pub fn as_integer<T>(self) -> Option<T>
    where
        T: TryFrom<BigInt> + TryFrom<i128>,
    {
        match self.0 {
            pallas::PlutusData::BigInt(big_int) => match big_int {
                pallas::BigInt::Int(int) => <T>::try_from(<i128>::from(int)).ok(),
                pallas::BigInt::BigUInt(bounded_bytes) => <T>::try_from(BigInt::from_bytes_be(
                    num_bigint::Sign::Plus,
                    &bounded_bytes,
                ))
                .ok(),
                pallas::BigInt::BigNInt(bounded_bytes) => <T>::try_from(BigInt::from_bytes_be(
                    num_bigint::Sign::Minus,
                    &bounded_bytes,
                ))
                .ok(),
            },
            _ => None,
        }
    }

    pub fn as_bytes(self) -> Option<Vec<u8>> {
        match self.0 {
            pallas::PlutusData::BoundedBytes(bounded_bytes) => Some(bounded_bytes.into()),
            _ => None,
        }
    }

    pub fn as_list(self) -> Option<Vec<Self>> {
        match self.0 {
            pallas::PlutusData::Array(array) => {
                let elems = match array {
                    pallas::MaybeIndefArray::Def(elems) => elems,
                    pallas::MaybeIndefArray::Indef(elems) => elems,
                };
                Some(elems.into_iter().map(|x| Self(x)).collect::<Vec<_>>())
            }
            _ => None,
        }
    }

    pub fn as_map(self) -> Option<Vec<(Self, Self)>> {
        match self.0 {
            pallas::PlutusData::Map(map) => {
                let items = match map {
                    uplc::KeyValuePairs::Def(items) => items,
                    uplc::KeyValuePairs::Indef(items) => items,
                };
                Some(
                    items
                        .into_iter()
                        .map(|(k, v)| (Self(k), Self(v)))
                        .collect::<Vec<_>>(),
                )
            }
            _ => None,
        }
    }

    pub fn as_constr(self) -> Option<(u64, Vec<Self>)> {
        match self.0 {
            pallas::PlutusData::Constr(pallas::Constr { tag, fields, .. }) => {
                let fields = match fields {
                    pallas::MaybeIndefArray::Def(fields) => fields,
                    pallas::MaybeIndefArray::Indef(fields) => fields,
                };
                let fields = fields.into_iter().map(|x| Self(x)).collect::<Vec<_>>();
                let ix = if tag == 102 {
                    9999
                } else if tag >= 1280 {
                    tag - 1280 + 7
                } else {
                    tag - 121
                };
                Some((ix, fields))
            }
            _ => None,
        }
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
