//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{Hash, cbor, pallas, pretty};
use anyhow::anyhow;
use num::{CheckedSub, Num, Zero};
use std::{
    collections::{BTreeMap, btree_map},
    fmt,
    fmt::Display,
};

#[derive(Debug, Clone, PartialEq, Eq)]
/// A multi-asset value, where 'Q' may typically be instantiated to either `u64` or `i64`
/// depending on whether it is represent an output value, or a mint value respectively.
pub struct Value<Quantity>(u64, BTreeMap<Hash<28>, BTreeMap<Vec<u8>, Quantity>>);

impl<Quantity: fmt::Debug + Copy> fmt::Display for Value<Quantity> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug_struct = f.debug_struct("Value");

        debug_struct.field("lovelace", &self.0);

        if !self.assets().is_empty() {
            debug_struct.field(
                "assets",
                &pretty::Fmt(|f: &mut fmt::Formatter<'_>| {
                    let mut outer = f.debug_map();
                    for (script_hash, assets) in &self.1 {
                        outer.entry(
                            &pretty::ViaDisplayNoAlloc(script_hash),
                            &pretty::Fmt(|f: &mut fmt::Formatter<'_>| {
                                let mut inner = f.debug_map();
                                for (name, qty) in assets {
                                    if let Ok(utf8) = str::from_utf8(name.as_slice()) {
                                        inner.entry(&pretty::ViaDisplayNoAlloc(utf8), qty);
                                    } else {
                                        inner.entry(
                                            &pretty::ViaDisplayNoAlloc(&hex::encode(
                                                name.as_slice(),
                                            )),
                                            qty,
                                        );
                                    }
                                }
                                inner.finish()
                            }),
                        );
                    }
                    outer.finish()
                }),
            );
        }

        debug_struct.finish()
    }
}

// -------------------------------------------------------------------- Inspecting

impl<Quantity> Value<Quantity> {
    pub fn lovelace(&self) -> u64 {
        self.0
    }

    pub fn assets(&self) -> &BTreeMap<Hash<28>, BTreeMap<Vec<u8>, Quantity>> {
        &self.1
    }

    pub fn is_empty(&self) -> bool {
        self.lovelace() == 0 && self.assets().is_empty()
    }
}

// -------------------------------------------------------------------- Building

impl<Quantity> Default for Value<Quantity> {
    fn default() -> Self {
        Self::new(0)
    }
}

impl<Quantity> Value<Quantity> {
    pub fn new(lovelace: u64) -> Self {
        Self(lovelace, BTreeMap::default())
    }

    pub fn with_lovelace(&mut self, lovelace: u64) -> &mut Self {
        self.0 = lovelace;
        self
    }
}

impl<Quantity: Num + CheckedSub + Copy + Display> Value<Quantity> {
    pub fn add(&mut self, rhs: &Self) -> &mut Self {
        self.0 += rhs.0;

        for (script_hash, assets) in &rhs.1 {
            self.1
                .entry(*script_hash)
                .and_modify(|lhs| {
                    for (asset_name, quantity) in assets {
                        lhs.entry(asset_name.clone())
                            .and_modify(|q| *q = q.add(*quantity))
                            .or_insert(*quantity);
                    }
                })
                .or_insert(assets.clone());
        }

        prune_null_values(&mut self.1);

        self
    }

    pub fn checked_sub(&mut self, rhs: &Self) -> anyhow::Result<&mut Self> {
        self.0 = self.0.checked_sub(rhs.0).ok_or_else(|| {
            anyhow!("insufficient lhs lovelace")
                .context(format!("lhs = {}, rhs = {}", self.0, rhs.0))
        })?;

        for (script_hash, assets) in &rhs.1 {
            match self.1.entry(*script_hash) {
                btree_map::Entry::Vacant(_) => {
                    return Err(anyhow!("script_hash={}", script_hash)
                        .context("insufficient lhs asset: unknown asset script_hash"));
                }
                btree_map::Entry::Occupied(mut lhs) => {
                    for (asset_name, quantity) in assets {
                        match lhs.get_mut().entry(asset_name.clone()) {
                            btree_map::Entry::Vacant(_) => {
                                return Err(anyhow!(
                                    "script hash={}, asset name={}",
                                    script_hash,
                                    display_asset_name(asset_name),
                                )
                                .context("insufficient lhs asset: unknown asset"));
                            }
                            btree_map::Entry::Occupied(mut q) => {
                                *q.get_mut() = q.get().checked_sub(quantity).ok_or_else(|| {
                                    anyhow!(
                                        "script hash={}, asset name={}",
                                        script_hash,
                                        display_asset_name(asset_name),
                                    )
                                    .context(format!(
                                        "lhs quantity={}, rhs quantity={}",
                                        q.get(),
                                        quantity,
                                    ))
                                    .context("insufficient lhs asset: insufficient quantity")
                                })?;
                            }
                        }
                    }
                }
            }
        }

        prune_null_values(&mut self.1);

        Ok(self)
    }
}

impl<Quantity: Zero> Value<Quantity> {
    pub fn with_assets<AssetName>(
        mut self,
        assets: impl IntoIterator<Item = (Hash<28>, impl IntoIterator<Item = (AssetName, Quantity)>)>,
    ) -> Self
    where
        AssetName: AsRef<[u8]>,
    {
        for (script_hash, inner) in assets.into_iter() {
            let mut inner = inner
                .into_iter()
                .filter_map(|(asset_name, quantity)| {
                    if quantity.is_zero() {
                        None
                    } else {
                        Some((Vec::from(asset_name.as_ref()), quantity))
                    }
                })
                .collect::<BTreeMap<_, _>>();

            self.1
                .entry(script_hash)
                .and_modify(|entry| entry.append(&mut inner))
                .or_insert(inner);
        }

        self
    }
}

// ------------------------------------------------------------ Converting (from)

impl From<&pallas::alonzo::Value> for Value<u64> {
    fn from(value: &pallas::alonzo::Value) -> Self {
        match value {
            pallas_primitives::alonzo::Value::Coin(lovelace) => {
                Self(*lovelace, BTreeMap::default())
            }
            pallas_primitives::alonzo::Value::Multiasset(lovelace, assets) => Self(
                *lovelace,
                assets
                    .iter()
                    .map(|(script_hash, inner)| {
                        (
                            Hash::from(script_hash),
                            inner
                                .iter()
                                .map(|(asset_name, quantity)| (asset_name.to_vec(), *quantity))
                                .collect(),
                        )
                    })
                    .collect(),
            ),
        }
    }
}

impl From<&pallas::Value> for Value<u64> {
    fn from(value: &pallas::Value) -> Self {
        match value {
            pallas_primitives::conway::Value::Coin(lovelace) => {
                Self(*lovelace, BTreeMap::default())
            }
            pallas_primitives::conway::Value::Multiasset(lovelace, assets) => {
                Self(*lovelace, from_multiasset(assets, |q| u64::from(q)))
            }
        }
    }
}

impl From<&pallas::Multiasset<pallas::NonZeroInt>> for Value<i64> {
    fn from(assets: &pallas::Multiasset<pallas::NonZeroInt>) -> Self {
        Self(0, from_multiasset(assets, |q| i64::from(q)))
    }
}

fn from_multiasset<Quantity: Copy, PositiveCoin: Copy>(
    assets: &pallas::Multiasset<PositiveCoin>,
    from_quantity: impl Fn(&PositiveCoin) -> Quantity,
) -> BTreeMap<Hash<28>, BTreeMap<Vec<u8>, Quantity>> {
    assets
        .iter()
        .map(|(script_hash, inner)| {
            (
                Hash::from(script_hash),
                inner
                    .iter()
                    .map(|(asset_name, quantity)| (asset_name.to_vec(), from_quantity(quantity)))
                    .collect(),
            )
        })
        .collect()
}

// -------------------------------------------------------------- Converting (to)

impl From<&Value<u64>> for pallas::Value {
    fn from(Value(lovelace, assets): &Value<u64>) -> Self {
        into_multiasset(assets, |quantity: &u64| {
            pallas::PositiveCoin::try_from(*quantity).ok()
        })
        .map(|assets| pallas::Value::Multiasset(*lovelace, assets))
        .unwrap_or_else(|| pallas::Value::Coin(*lovelace))
    }
}

impl From<&Value<i64>> for Option<pallas::Multiasset<pallas::NonZeroInt>> {
    fn from(value @ Value(lovelace, assets): &Value<i64>) -> Self {
        debug_assert!(
            *lovelace == 0,
            "somehow found a mint value with a non-zero Ada quantity: {value:#?}"
        );
        into_multiasset(assets, |quantity: &i64| {
            pallas::NonZeroInt::try_from(*quantity).ok()
        })
    }
}

/// Convert a multi-asset map into a Pallas' Multiasset. Returns 'None' when empty once pruned of
/// any null quantities values.
fn into_multiasset<Quantity: Copy, PositiveCoin: Copy>(
    assets: &BTreeMap<Hash<28>, BTreeMap<Vec<u8>, Quantity>>,
    from_quantity: impl Fn(&Quantity) -> Option<PositiveCoin>,
) -> Option<pallas::Multiasset<PositiveCoin>> {
    pallas::NonEmptyKeyValuePairs::from_vec(
        assets
            .iter()
            .filter_map(|(script_hash, inner)| {
                pallas::NonEmptyKeyValuePairs::from_vec(
                    inner
                        .iter()
                        .filter_map(|(asset_name, quantity)| {
                            from_quantity(quantity)
                                .map(|quantity| (pallas::Bytes::from(asset_name.clone()), quantity))
                        })
                        .collect::<Vec<_>>(),
                )
                .map(|inner| (pallas::Hash::from(script_hash), inner))
            })
            .collect::<Vec<_>>(),
    )
}

// -------------------------------------------------------------------- Encoding

impl<C> cbor::Encode<C> for Value<u64> {
    fn encode<W: cbor::encode::write::Write>(
        &self,
        e: &mut cbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), cbor::encode::Error<W::Error>> {
        pallas::Value::from(self).encode(e, ctx)
    }
}

impl<'d, C> cbor::Decode<'d, C> for Value<u64> {
    fn decode(d: &mut cbor::Decoder<'d>, ctx: &mut C) -> Result<Self, cbor::decode::Error> {
        let value: pallas::Value = d.decode_with(ctx)?;
        Ok(Self::from(&value))
    }
}

// -------------------------------------------------------------------- Internal

fn prune_null_values<Quantity: Zero>(value: &mut BTreeMap<Hash<28>, BTreeMap<Vec<u8>, Quantity>>) {
    let mut script_hashes_to_remove = Vec::new();

    for (script_hash, assets) in value.iter_mut() {
        let mut assets_to_remove = Vec::new();

        for (asset_name, quantity) in assets.iter() {
            if quantity.is_zero() {
                assets_to_remove.push(asset_name.clone());
            }
        }

        for asset_name in assets_to_remove {
            assets.remove(&asset_name);
        }

        if assets.is_empty() {
            script_hashes_to_remove.push(*script_hash)
        }
    }

    for script_hash in script_hashes_to_remove {
        value.remove(&script_hash);
    }
}

fn display_asset_name(asset_name: &[u8]) -> String {
    if let Ok(utf8) = str::from_utf8(asset_name) {
        utf8.to_string()
    } else {
        hex::encode(asset_name)
    }
}

#[cfg(test)]
mod tests {
    use super::Value;
    use crate::value;

    #[test]
    fn display_only_lovelace() {
        let value: Value<u64> = Value::new(42);
        assert_eq!(value.to_string(), "Value { lovelace: 42 }")
    }

    #[test]
    fn display_value_with_assets() {
        let value: Value<u64> = value!(
            6687232,
            (
                "279c909f348e533da5808898f87f9a14bb2c3dfbbacccd631d927a3f",
                "534e454b",
                1376
            ),
            (
                "a0028f350aaabe0545fdcb56b039bfb08e4bb4d8c4d7c3c7d481c235",
                "484f534b59",
                134468443
            ),
            (
                "f350aaabe0545fdcb56b039bfb08e4bb4d8c4d7c3c7d481c2a002835",
                "b4d8cdcb5b039b",
                1
            ),
        );
        assert_eq!(
            value.to_string(),
            "Value { \
                lovelace: 6687232, \
                assets: {\
                    279c909f348e533da5808898f87f9a14bb2c3dfbbacccd631d927a3f: {SNEK: 1376}, \
                    a0028f350aaabe0545fdcb56b039bfb08e4bb4d8c4d7c3c7d481c235: {HOSKY: 134468443}, \
                    f350aaabe0545fdcb56b039bfb08e4bb4d8c4d7c3c7d481c2a002835: {b4d8cdcb5b039b: 1}\
                } \
            }",
        )
    }
}
