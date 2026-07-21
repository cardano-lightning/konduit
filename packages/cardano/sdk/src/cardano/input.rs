//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{Hash, cbor, pallas};
use anyhow::anyhow;
use std::{fmt, str::FromStr, sync::Arc};

#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};
#[cfg(feature = "serde")]
use serde_with::{hex::Hex, serde_as};

/// A reference to a past transaction output.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Input(Arc<pallas::TransactionInput>);

impl fmt::Display for Input {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Input({}#{})", &self.0.transaction_id, self.0.index)
    }
}

// -------------------------------------------------------------------- Building
impl Input {
    /// See also [`input!`](crate::input).
    pub fn new(transaction_id: Hash<32>, output_index: u64) -> Self {
        Self(Arc::new(pallas::TransactionInput {
            transaction_id: pallas::Hash::from(transaction_id),
            index: output_index,
        }))
    }
}

// ------------------------------------------------------------------------ serde
#[cfg(feature = "serde")]
#[serde_as]
#[derive(Serialize, Deserialize)]
struct InputRaw {
    #[serde_as(as = "Hex")]
    transaction_id: [u8; 32],
    index: u64,
}

#[cfg(feature = "serde")]
impl Serialize for Input {
    fn serialize<Ser: Serializer>(&self, serializer: Ser) -> Result<Ser::Ok, Ser::Error> {
        // ASSUMPTION: `pallas::Hash<32>` exposes its bytes via `AsRef<[u8]>`
        // (or similar) — adjust the extraction if the real accessor differs.
        let bytes: [u8; 32] = self
            .0
            .transaction_id
            .as_ref()
            .try_into()
            .map_err(|_| serde::ser::Error::custom("transaction_id was not 32 bytes"))?;
        InputRaw {
            transaction_id: bytes,
            index: self.0.index,
        }
        .serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for Input {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = InputRaw::deserialize(deserializer)?;
        Ok(Self::new(Hash::from(raw.transaction_id), raw.index))
    }
}

// ------------------------------------------------------------------ Inspecting

impl Input {
    pub fn transaction_id(&self) -> Hash<32> {
        Hash::from(self.0.transaction_id)
    }

    pub fn output_index(&self) -> u64 {
        self.0.index
    }
}

// ----------------------------------------------------------- Converting (from)

impl From<pallas::TransactionInput> for Input {
    fn from(i: pallas::TransactionInput) -> Self {
        Input(Arc::new(i))
    }
}

impl FromStr for Input {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        let mut split = s.split("#");

        let transaction_id = split.next().ok_or(anyhow!("missing transaction id"))?;

        let index = split.next().ok_or(anyhow!("missing output index"))?;

        if split.next().is_some() {
            return Err(anyhow!("leftovers after output index"));
        }

        Ok(Self::new(
            <Hash<32>>::try_from(transaction_id)?,
            index.parse::<u64>()?,
        ))
    }
}

// ------------------------------------------------------------- Converting (to)

impl From<Input> for pallas::TransactionInput {
    fn from(i: Input) -> Self {
        Arc::unwrap_or_clone(i.0)
    }
}

// -------------------------------------------------------------------- Encoding

impl<C> cbor::Encode<C> for Input {
    fn encode<W: cbor::encode::write::Write>(
        &self,
        e: &mut cbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), cbor::encode::Error<W::Error>> {
        e.encode_with(self.0.as_ref(), ctx)?;
        Ok(())
    }
}

impl<'d, C> cbor::Decode<'d, C> for Input {
    fn decode(d: &mut cbor::Decoder<'d>, ctx: &mut C) -> Result<Self, cbor::decode::Error> {
        Ok(Self(Arc::new(d.decode_with(ctx)?)))
    }
}

#[cfg(any(test, feature = "test-utils"))]
pub mod tests {
    use crate::{Input, any, hash};
    use proptest::prelude::*;

    // -------------------------------------------------------------- Unit tests

    #[test]
    fn display_input() {
        assert_eq!(
            Input::new(
                hash!("702206530b2e1566e90b3aec753bd0abbf397842bd5421e0c3d23ed10167b3ce"),
                42,
            )
            .to_string(),
            "Input(702206530b2e1566e90b3aec753bd0abbf397842bd5421e0c3d23ed10167b3ce#42)",
        );
    }

    // -------------------------------------------------------------- Generators

    pub mod generators {
        use super::*;

        prop_compose! {
            pub fn input()(id in any::hash32(), ix in any::<u64>()) -> Input {
                Input::new(id, ix)
            }
        }
    }
}
