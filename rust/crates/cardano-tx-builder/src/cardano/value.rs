//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{Hash, cbor, pallas};
use anyhow::anyhow;
use num::{CheckedSub, Num, Zero};
use std::{
    collections::{BTreeMap, btree_map},
    fmt::Display,
};

#[derive(Debug, Clone)]
/// A multi-asset value, where 'Q' may typically be instantiated to either `u64` or `i64`
/// depending on whether it is represent an output value, or a mint value respectively.
pub struct Value<Q>(u64, BTreeMap<Hash<28>, BTreeMap<Vec<u8>, Q>>);

// -------------------------------------------------------------------- Inspecting

impl<Q> Value<Q> {
    pub fn lovelace(&self) -> u64 {
        self.0
    }

    pub fn assets(&self) -> &BTreeMap<Hash<28>, BTreeMap<Vec<u8>, Q>> {
        &self.1
    }

    pub fn is_empty(&self) -> bool {
        self.lovelace() == 0 && self.assets().is_empty()
    }
}

// -------------------------------------------------------------------- Building

impl<Q> Default for Value<Q> {
    fn default() -> Self {
        Self::new(0)
    }
}

impl<Q> Value<Q> {
    pub fn new(lovelace: u64) -> Self {
        Self(lovelace, BTreeMap::default())
    }
}

impl<Q: Num + CheckedSub + Copy + Display> Value<Q> {
    pub fn add(&mut self, rhs: &Self) -> &mut Self {
        self.0 += rhs.0;

        for (policy, assets) in &rhs.1 {
            self.1
                .entry(*policy)
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

        for (policy, assets) in &rhs.1 {
            match self.1.entry(*policy) {
                btree_map::Entry::Vacant(_) => {
                    return Err(anyhow!("insufficient lhs asset: unknown asset policy")
                        .context(format!("policy={:?}", policy)));
                }
                btree_map::Entry::Occupied(mut lhs) => {
                    for (asset_name, quantity) in assets {
                        match lhs.get_mut().entry(asset_name.clone()) {
                            btree_map::Entry::Vacant(_) => {
                                return Err(anyhow!("insufficient lhs asset: unknown asset")
                                    .context(format!(
                                        "policy={:?}, asset_name={:?}",
                                        policy, asset_name
                                    )));
                            }
                            btree_map::Entry::Occupied(mut q) => {
                                *q.get_mut() = q.get().checked_sub(quantity).ok_or_else(|| {
                                    anyhow!("insufficient lhs asset: insufficient quantity")
                                        .context(format!(
                                            "policy={:?}, asset_name={:?}",
                                            policy, asset_name
                                        ))
                                        .context(format!(
                                            "lhs quantity={}, rhs quantity={}",
                                            q.get(),
                                            quantity,
                                        ))
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

impl<Q: Zero> Value<Q> {
    pub fn with_assets(
        mut self,
        assets: impl IntoIterator<Item = (Hash<28>, impl IntoIterator<Item = (Vec<u8>, Q)>)>,
    ) -> Self {
        for (policy, inner) in assets.into_iter() {
            let mut inner = inner
                .into_iter()
                .map(|(asset_name, quantity)| {
                    assert!(
                        !quantity.is_zero(),
                        "null quantity of asset {}.{}",
                        policy,
                        hex::encode(&asset_name)
                    );
                    (asset_name, quantity)
                })
                .collect::<BTreeMap<_, _>>();

            self.1
                .entry(policy)
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
                    .map(|(policy, inner)| {
                        (
                            Hash::from(policy),
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

fn from_multiasset<Q: Copy, P: Copy>(
    assets: &pallas::Multiasset<P>,
    from_quantity: impl Fn(&P) -> Q,
) -> BTreeMap<Hash<28>, BTreeMap<Vec<u8>, Q>> {
    assets
        .iter()
        .map(|(policy, inner)| {
            (
                Hash::from(policy),
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
        assert!(
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
fn into_multiasset<Q: Copy, P: Copy>(
    assets: &BTreeMap<Hash<28>, BTreeMap<Vec<u8>, Q>>,
    from_quantity: impl Fn(&Q) -> Option<P>,
) -> Option<pallas::Multiasset<P>> {
    pallas::NonEmptyKeyValuePairs::from_vec(
        assets
            .iter()
            .filter_map(|(policy, inner)| {
                pallas::NonEmptyKeyValuePairs::from_vec(
                    inner
                        .iter()
                        .filter_map(|(asset_name, quantity)| {
                            from_quantity(quantity)
                                .map(|quantity| (pallas::Bytes::from(asset_name.clone()), quantity))
                        })
                        .collect::<Vec<_>>(),
                )
                .map(|inner| (pallas::Hash::from(policy), inner))
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

fn prune_null_values<Q: Zero>(value: &mut BTreeMap<Hash<28>, BTreeMap<Vec<u8>, Q>>) {
    let mut policies_to_remove = Vec::new();

    for (policy, assets) in value.iter_mut() {
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
            policies_to_remove.push(*policy)
        }
    }

    for policy in policies_to_remove {
        value.remove(&policy);
    }
}
