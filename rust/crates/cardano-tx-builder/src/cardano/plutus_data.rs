//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{cbor, cbor::ToCbor, pallas};
use thiserror::Error;

use num::ToPrimitive;
use num_bigint::BigInt;
use std::{borrow::Cow, fmt, str::FromStr};

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
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PlutusData<'a>(Cow<'a, pallas::PlutusData>);

impl<'a> fmt::Display for PlutusData<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CBOR({})", hex::encode(self.to_cbor()))
    }
}

// -------------------------------------------------------------------- Building

impl<'a> PlutusData<'a> {
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

        Self(Cow::Owned(match i.to_i128().map(pallas::Int::try_from) {
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
        }))
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
        Self(Cow::Owned(pallas::PlutusData::BoundedBytes(
            pallas::BoundedBytes::from(bytes.as_ref().to_vec()),
        )))
    }

    /// Construct an arbitrarily-sized list of [`self::PlutusData`] values.
    ///
    /// # examples
    ///
    /// ```rust
    /// # use cardano_tx_builder::PlutusData;
    ///
    /// assert_eq!(
    ///     format!("{}", PlutusData::list::<PlutusData>([])),
    ///     "CBOR(80)"
    /// );
    ///
    /// assert_eq!(
    ///     format!("{}", PlutusData::list([b"foo", b"bar"])),
    ///     "CBOR(9f43666f6f43626172ff)"
    /// );
    ///
    /// assert_eq!(
    ///     format!("{}", PlutusData::list([
    ///         PlutusData::bytes(b"foo"),
    ///         PlutusData::list([1, 2]),
    ///     ])),
    ///     "CBOR(9f43666f6f9f0102ffff)"
    /// );
    /// ```
    pub fn list<T: Into<Self>>(elems: impl IntoIterator<Item = T>) -> Self {
        let elems = elems
            .into_iter()
            .map(Into::<Self>::into)
            .map(pallas::PlutusData::from)
            .collect::<Vec<_>>();

        Self(Cow::Owned(pallas::PlutusData::Array(if elems.is_empty() {
            pallas::MaybeIndefArray::Def(elems)
        } else {
            pallas::MaybeIndefArray::Indef(elems)
        })))
    }

    /// Construct an arbitrarily-sized list of [`self::PlutusData`] values.
    ///
    /// # examples
    ///
    /// ```rust
    /// # use cardano_tx_builder::PlutusData;
    ///
    /// assert_eq!(
    ///     format!("{}", PlutusData::map::<PlutusData, PlutusData>([])),
    ///     "CBOR(a0)"
    /// );
    ///
    /// assert_eq!(
    ///     format!(
    ///         "{}",
    ///         PlutusData::map([
    ///             (b"FOO", 1),
    ///             (b"BAR", 2),
    ///         ]),
    ///     ),
    ///     "CBOR(a243464f4f014342415202)"
    /// );
    pub fn map<K: Into<Self>, V: Into<Self>>(kvs: impl IntoIterator<Item = (K, V)>) -> Self {
        let kvs = kvs
            .into_iter()
            .map(|(k, v)| {
                (
                    pallas::PlutusData::from(Into::<Self>::into(k)),
                    pallas::PlutusData::from(Into::<Self>::into(v)),
                )
            })
            .collect::<Vec<_>>();

        Self(Cow::Owned(pallas::PlutusData::Map(
            pallas::KeyValuePairs::from(kvs),
        )))
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
    ///     format!("{}", PlutusData::constr::<PlutusData>(0, [])),
    ///     "CBOR(d87980)"
    /// );
    ///
    /// assert_eq!(
    ///     format!(
    ///         "{}",
    ///         PlutusData::constr(0, [
    ///             PlutusData::constr::<PlutusData>(1, []),
    ///             PlutusData::integer(1337),
    ///         ]),
    ///     ),
    ///     "CBOR(d8799fd87a80190539ff)"
    /// );
    /// ```
    pub fn constr<T: Into<Self>>(ix: u64, fields: impl IntoIterator<Item = T>) -> Self {
        let fields = fields
            .into_iter()
            .map(Into::<Self>::into)
            .map(pallas::PlutusData::from)
            .collect::<Vec<_>>();

        let fields = if fields.is_empty() {
            pallas::MaybeIndefArray::Def(fields)
        } else {
            pallas::MaybeIndefArray::Indef(fields)
        };

        // NOTE: see https://github.com/input-output-hk/plutus/blob/9538fc9829426b2ecb0628d352e2d7af96ec8204/plutus-core/plutus-core/src/PlutusCore/Data.hs#L139-L155
        Self(Cow::Owned(if ix < 7 {
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
        }))
    }
}

// ---------------------------------------------------------------------- Macros

#[macro_export]
/// A handy macro for constructing [`PlutusData`](crate::PlutusData) constructors from a known set
/// of fields. The macro is variadic. The first argument refers to the constructor variant index,
/// while other arguments indicates the constructor fields.
///
/// ```rust
/// use cardano_tx_builder::{PlutusData, constr};
/// assert_eq!(
///   constr!(1),
///   PlutusData::constr::<PlutusData>(1, []),
/// );
/// ```
///
/// ```rust
/// use cardano_tx_builder::{PlutusData, constr};
/// assert_eq!(
///   constr!(0, b"foo"),
///   PlutusData::constr(0, [b"foo"]),
/// );
/// ```
///
/// ```rust
/// use cardano_tx_builder::{PlutusData, constr};
/// assert_eq!(
///   constr!(0, 42, b"foo"),
///   PlutusData::constr::<PlutusData>(0, [42.into(), b"foo".into()]),
/// );
/// ```
///
macro_rules! constr {
    ($ix:expr $(,)?) => {
        $crate::PlutusData::constr::<$crate::PlutusData>($ix, [])
    };

    // comma-separated arguments: constr!(ix, a, b, c)
    ($ix:expr $(, $field:expr)+ $(,)?) => {
        $crate::PlutusData::constr($ix, [ $( ::core::convert::Into::<$crate::PlutusData>::into($field) ),* ])
    };
}

// ------------------------------------------------------------------ Inspecting

impl<'a> PlutusData<'a> {
    pub fn as_integer<T>(&'a self) -> Option<T>
    where
        T: TryFrom<BigInt>,
    {
        match self.0.as_ref() {
            pallas::PlutusData::BigInt(big_int) => from_pallas_bigint(big_int).try_into().ok(),
            _ => None,
        }
    }

    pub fn as_bytes(&'a self) -> Option<&'a [u8]> {
        match self.0.as_ref() {
            pallas::PlutusData::BoundedBytes(bounded_bytes) => Some(bounded_bytes.as_slice()),
            _ => None,
        }
    }

    pub fn as_list(&'a self) -> Option<impl Iterator<Item = Self>> {
        match self.0.as_ref() {
            pallas::PlutusData::Array(array) => Some(from_pallas_array(array)),
            _ => None,
        }
    }

    pub fn as_map(&'a self) -> Option<impl Iterator<Item = (Self, Self)>> {
        match self.0.as_ref() {
            pallas::PlutusData::Map(map) => Some(from_pallas_map(map)),
            _ => None,
        }
    }

    pub fn as_constr(&'a self) -> Option<(u64, impl Iterator<Item = Self>)> {
        match self.0.as_ref() {
            pallas::PlutusData::Constr(constr) => Some(from_pallas_constr(constr)),
            _ => None,
        }
    }
}

// --------------------------------------------------------------------- Helpers

fn from_pallas_bigint(big_int: &pallas::BigInt) -> BigInt {
    match big_int {
        pallas::BigInt::Int(int) => BigInt::from(i128::from(*int)),
        pallas::BigInt::BigUInt(bounded_bytes) => {
            BigInt::from_bytes_be(num_bigint::Sign::Plus, bounded_bytes)
        }
        pallas::BigInt::BigNInt(bounded_bytes) => {
            BigInt::from_bytes_be(num_bigint::Sign::Minus, bounded_bytes)
        }
    }
}

fn from_pallas_array<'a>(
    array: &'a pallas::MaybeIndefArray<pallas::PlutusData>,
) -> impl Iterator<Item = PlutusData<'a>> {
    match array {
        pallas::MaybeIndefArray::Def(elems) => elems,
        pallas::MaybeIndefArray::Indef(elems) => elems,
    }
    .iter()
    .map(|x| PlutusData(Cow::Borrowed(x)))
}

fn from_pallas_map<'a>(
    map: &'a pallas::KeyValuePairs<pallas::PlutusData, pallas::PlutusData>,
) -> impl Iterator<Item = (PlutusData<'a>, PlutusData<'a>)> {
    match map {
        pallas::KeyValuePairs::Def(items) => items,
        pallas::KeyValuePairs::Indef(items) => items,
    }
    .iter()
    .map(|(k, v)| (PlutusData(Cow::Borrowed(k)), PlutusData(Cow::Borrowed(v))))
}

fn from_pallas_constr<'a>(
    constr: &'a pallas::Constr<pallas::PlutusData>,
) -> (u64, impl Iterator<Item = PlutusData<'a>>) {
    let fields = match &constr.fields {
        pallas::MaybeIndefArray::Def(fields) => fields,
        pallas::MaybeIndefArray::Indef(fields) => fields,
    }
    .iter()
    .map(|x| PlutusData(Cow::Borrowed(x)));

    let ix = if constr.tag == 102 {
        constr
            .any_constructor
            .expect("'any_constructor' was 'None' but 'tag' was set to 102? This is absurd.")
    } else if constr.tag >= 1280 {
        constr.tag - 1280 + 7
    } else {
        constr.tag - 121
    };

    (ix, fields)
}

// ----------------------------------------------------------- Converting (from)

impl From<pallas::PlutusData> for PlutusData<'static> {
    fn from(data: pallas::PlutusData) -> Self {
        Self(Cow::Owned(data))
    }
}

#[macro_export]
macro_rules! impl_from_int {
    ($($t:ty),+ $(,)?) => {
        $(
            impl From<$t> for PlutusData<'static> {
                #[inline]
                fn from(int: $t) -> Self {
                    Self::integer(int)
                }
            }
        )+
    };
}

impl_from_int!(
    u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, BigInt
);

impl From<&str> for PlutusData<'static> {
    fn from(bytes: &str) -> Self {
        Self::bytes(bytes.as_bytes())
    }
}

impl From<String> for PlutusData<'static> {
    fn from(bytes: String) -> Self {
        Self::bytes(bytes.as_bytes())
    }
}

impl From<Vec<u8>> for PlutusData<'static> {
    fn from(bytes: Vec<u8>) -> Self {
        Self::bytes(bytes)
    }
}

impl From<&[u8]> for PlutusData<'static> {
    fn from(bytes: &[u8]) -> Self {
        Self::bytes(bytes)
    }
}

impl<const T: usize> From<[u8; T]> for PlutusData<'static> {
    fn from(bytes: [u8; T]) -> Self {
        Self::bytes(bytes)
    }
}

impl<const T: usize> From<&[u8; T]> for PlutusData<'static> {
    fn from(bytes: &[u8; T]) -> Self {
        Self::bytes(bytes)
    }
}

// ------------------------------------------------------------- Converting (to)

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PlutusType {
    Integer,
    Bytes,
    List,
    Map,
    Constr,
}

#[derive(Debug)]
pub struct ParsePlutusDataTypeError;

impl fmt::Display for PlutusType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlutusType::Integer => write!(f, "Integer"),
            PlutusType::Bytes => write!(f, "Bytes"),
            PlutusType::List => write!(f, "List"),
            PlutusType::Map => write!(f, "Map"),
            PlutusType::Constr => write!(f, "Constr"),
        }
    }
}

impl FromStr for PlutusType {
    type Err = ParsePlutusDataTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Integer" => Ok(PlutusType::Integer),
            "Bytes" => Ok(PlutusType::Bytes),
            "List" => Ok(PlutusType::List),
            "Map" => Ok(PlutusType::Map),
            "Constr" => Ok(PlutusType::Constr),
            _ => Err(ParsePlutusDataTypeError),
        }
    }
}

impl<'a> From<&PlutusData<'a>> for PlutusType {
    fn from(value: &PlutusData) -> Self {
        if let Some(_) = value.as_constr() {
            PlutusType::Constr
        } else if let Some(_) = value.as_map() {
            PlutusType::Map
        } else if let Some(_) = value.as_list() {
            PlutusType::List
        } else if let Some(_) = value.as_bytes() {
            PlutusType::Bytes
        } else if let Some(_) = value.as_integer::<i128>() {
            PlutusType::Integer
        } else {
            panic!("Impossible!")
        }
    }
}

/// A first approximation of errors
#[derive(Debug, Error)]
pub enum PlutusDecodeError {
    #[error("Expected plutus type {0}, found {1}")]
    Type(PlutusType, PlutusType),
    #[error("Expected constr tag {0}, found {1}")]
    Tag(u64, u64),
    #[error("Expected list len {0}, found {1}")]
    ListLen(usize, usize),
    #[error("Expected bytes len {0}, found {1}")]
    BytesLen(usize, usize),
    // TODO :: should we also check in coercion?
    // #[error("Expected int within type safe range")]
    // IntSize,
}

impl From<PlutusData<'_>> for pallas::PlutusData {
    fn from(data: PlutusData<'_>) -> Self {
        data.0.into_owned()
    }
}

#[macro_export]
macro_rules! impl_try_into_int {
    ($($t:ty),+ $(,)?) => {
        $(
            impl<'a> TryFrom<&'a PlutusData<'a>> for $t {
                type Error = PlutusDecodeError;

                            #[inline]
                fn try_from(data: &'a PlutusData<'a>) -> Result<Self, Self::Error> {
                    data.as_integer().ok_or(PlutusDecodeError::Type(PlutusType::Integer, PlutusType::from(data)))
                }
            }
        )+
    };
}

impl_try_into_int!(
    u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, BigInt,
);

impl<'a> TryFrom<&'a PlutusData<'a>> for &'a [u8] {
    type Error = PlutusDecodeError;

    fn try_from(data: &'a PlutusData<'a>) -> Result<Self, Self::Error> {
        data.as_bytes().ok_or(PlutusDecodeError::Type(
            PlutusType::Bytes,
            PlutusType::from(data),
        ))
    }
}

impl<'a, const T: usize> TryFrom<&'a PlutusData<'a>> for &'a [u8; T] {
    type Error = PlutusDecodeError;

    fn try_from(data: &'a PlutusData<'a>) -> Result<Self, Self::Error> {
        let vec = data.as_bytes().ok_or(PlutusDecodeError::Type(
            PlutusType::Bytes,
            PlutusType::from(data),
        ))?;
        vec.try_into()
            .map_err(|_| PlutusDecodeError::BytesLen(T, vec.len()))
    }
}

impl<'a> TryFrom<&'a PlutusData<'a>> for Vec<PlutusData<'a>> {
    type Error = PlutusDecodeError;

    fn try_from(data: &'a PlutusData<'a>) -> Result<Self, Self::Error> {
        Ok(data
            .as_list()
            .ok_or(PlutusDecodeError::Type(
                PlutusType::List,
                PlutusType::from(data),
            ))?
            .collect())
    }
}

impl<'a, const T: usize> TryFrom<&'a PlutusData<'a>> for [PlutusData<'a>; T] {
    type Error = PlutusDecodeError;

    fn try_from(data: &'a PlutusData<'a>) -> Result<Self, Self::Error> {
        let list: Vec<PlutusData<'_>> = data.try_into()?;
        <[PlutusData<'_>; T]>::try_from(list)
            .map_err(|list| PlutusDecodeError::ListLen(T, list.len()))
    }
}

impl<'a> TryFrom<&'a PlutusData<'a>> for Vec<(PlutusData<'a>, PlutusData<'a>)> {
    type Error = PlutusDecodeError;

    fn try_from(data: &'a PlutusData<'a>) -> Result<Self, Self::Error> {
        Ok(data
            .as_map()
            .ok_or(PlutusDecodeError::Type(
                PlutusType::List,
                PlutusType::from(data),
            ))?
            .collect())
    }
}

impl<'a> TryFrom<&'a PlutusData<'a>> for (u64, Vec<PlutusData<'a>>) {
    type Error = PlutusDecodeError;

    fn try_from(data: &'a PlutusData<'a>) -> Result<Self, Self::Error> {
        let (ix, fields) = data.as_constr().ok_or(PlutusDecodeError::Type(
            PlutusType::List,
            PlutusType::from(data),
        ))?;
        Ok((ix, fields.collect()))
    }
}

impl<'a, const T: usize> TryFrom<&'a PlutusData<'a>> for (u64, [PlutusData<'a>; T]) {
    type Error = PlutusDecodeError;

    fn try_from(data: &'a PlutusData<'a>) -> Result<Self, Self::Error> {
        let (ix, fields) = data.as_constr().ok_or(PlutusDecodeError::Type(
            PlutusType::List,
            PlutusType::from(data),
        ))?;
        Ok((
            ix,
            <[PlutusData<'a>; T]>::try_from(fields.collect::<Vec<_>>())
                .map_err(|list| PlutusDecodeError::ListLen(T, list.len()))?,
        ))
    }
}

// -------------------------------------------------------------------- Encoding

impl<C> cbor::Encode<C> for PlutusData<'_> {
    fn encode<W: cbor::encode::write::Write>(
        &self,
        e: &mut cbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), cbor::encode::Error<W::Error>> {
        e.encode_with(self.0.as_ref(), ctx)?;
        Ok(())
    }
}

impl<'d, C> cbor::Decode<'d, C> for PlutusData<'static> {
    fn decode(d: &mut cbor::Decoder<'d>, ctx: &mut C) -> Result<Self, cbor::decode::Error> {
        Ok(Self(Cow::Owned(d.decode_with(ctx)?)))
    }
}
