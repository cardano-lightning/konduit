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
/// A multi-asset value, generic in its asset quantities.
///
/// `Quantity` will typically be instantiated to either `u64` or `i64` depending on whether it is
/// represent an output value, or a mint value respectively.
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

// -------------------------------------------------------------------- Building

impl<Quantity> Default for Value<Quantity> {
    fn default() -> Self {
        Self::new(0)
    }
}

impl<Quantity> Value<Quantity> {
    /// Construct a new value holding only lovelaces. Use [`Self::with_assets`] to add assets if
    /// needed.
    ///
    /// # examples
    ///
    /// ```rust
    /// # use cardano_sdk::{Value, hash, value};
    /// assert_eq!(Value::<u64>::new(123456789), value!(123_456_789));
    /// ```
    ///
    /// See also [`value!`](crate::value!).
    pub fn new(lovelace: u64) -> Self {
        Self(lovelace, BTreeMap::default())
    }

    /// Replace the amount of lovelaces currently attached to the value.
    ///
    /// ```rust
    /// # use cardano_sdk::{Value};
    /// assert_eq!(
    ///     Value::<u64>::new(14).with_lovelace(42).lovelace(),
    ///     42,
    /// )
    /// ```
    pub fn with_lovelace(&mut self, lovelace: u64) -> &mut Self {
        self.0 = lovelace;
        self
    }
}

impl<Quantity: Zero> Value<Quantity> {
    /// Attach native assets to the value, replacing any existing assets already set on the value.
    ///
    /// # examples
    ///
    /// ```rust
    /// # use cardano_sdk::{Value, hash, value};
    /// assert_eq!(
    ///     Value::new(123456789)
    ///         .with_assets([
    ///             (
    ///                 hash!("279c909f348e533da5808898f87f9a14bb2c3dfbbacccd631d927a3f"),
    ///                 [( b"SNEK", 1_000_000)]
    ///             ),
    ///         ]),
    ///     value!(
    ///         123_456_789,
    ///         (
    ///             "279c909f348e533da5808898f87f9a14bb2c3dfbbacccd631d927a3f",
    ///             "534e454b",
    ///             1_000_000,
    ///         ),
    ///     ),
    /// );
    /// ```
    pub fn with_assets<AssetName>(
        mut self,
        assets: impl IntoIterator<Item = (Hash<28>, impl IntoIterator<Item = (AssetName, Quantity)>)>,
    ) -> Self
    where
        AssetName: AsRef<[u8]>,
    {
        with_assets(&mut self, assets);
        self
    }
}

impl<Quantity: Num + CheckedSub + Copy + Display> Value<Quantity> {
    /// Add two values together, removing any entries that results in a null quantity. The latter
    /// is possible when quantities can take negative values (e.g. [`i64`]).
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

    /// Subtract the right-hand side argument from the current value; returning an error if there's
    /// not enough of a particular quantity on the left-hand side.
    /// # examples
    ///
    /// ```rust
    /// # use cardano_sdk::{Value};
    /// assert!(Value::<u64>::new(10).checked_sub(&Value::new(20)).is_err());
    /// ```
    ///
    /// ```rust
    /// # use cardano_sdk::{Value, hash};
    /// let lhs: Value<u64> =
    ///   Value::default()
    ///     .with_assets([
    ///       (
    ///           hash!("b558ea5ecfa2a6e9701dab150248e94104402f789c090426eb60eb60"),
    ///           vec![( Vec::from(b"Snekkie0903"), 1), ( Vec::from(b"Snekkie3556"), 1)],
    ///       ),
    ///       (
    ///           hash!("a0028f350aaabe0545fdcb56b039bfb08e4bb4d8c4d7c3c7d481c235"),
    ///           vec![( Vec::from(b"HOSKY"), 42_000_000)],
    ///       ),
    ///     ]);
    ///
    /// assert!(lhs.clone().checked_sub(&lhs).is_ok_and(|value| value == &Value::default()));
    ///
    /// let rhs_missing_asset =
    ///   Value::default()
    ///     .with_assets([
    ///       (
    ///           hash!("b558ea5ecfa2a6e9701dab150248e94104402f789c090426eb60eb60"),
    ///           vec![( Vec::from(b"Snekkie9999"), 1)],
    ///       ),
    ///       (
    ///           hash!("a0028f350aaabe0545fdcb56b039bfb08e4bb4d8c4d7c3c7d481c235"),
    ///           vec![( Vec::from(b"HOSKY"), 42_000_000)],
    ///       ),
    ///     ]);
    ///
    /// assert!(lhs.clone().checked_sub(&rhs_missing_asset).is_err());
    ///
    /// let rhs_missing_script =
    ///   Value::default()
    ///     .with_assets([
    ///       (
    ///           hash!("dcb56b039bfb08e4bb4d8c4d7c3c7d481c235a0028f350aaabe0545f"),
    ///           vec![( Vec::from(b"HOSKY"), 42_000_000)],
    ///       ),
    ///     ]);
    ///
    /// assert!(lhs.clone().checked_sub(&rhs_missing_script).is_err());
    ///
    /// let rhs_missing_quantity =
    ///   Value::default()
    ///     .with_assets([
    ///       (
    ///           hash!("b558ea5ecfa2a6e9701dab150248e94104402f789c090426eb60eb60"),
    ///           vec![( Vec::from(b"Snekkie0903"), 2)],
    ///       ),
    ///     ]);
    ///
    /// assert!(lhs.clone().checked_sub(&rhs_missing_quantity).is_err());
    /// ```
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

fn with_assets<AssetName, Quantity: Zero>(
    value: &mut Value<Quantity>,
    assets: impl IntoIterator<Item = (Hash<28>, impl IntoIterator<Item = (AssetName, Quantity)>)>,
) where
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

        value
            .1
            .entry(script_hash)
            .and_modify(|entry| entry.append(&mut inner))
            .or_insert(inner);
    }
}

// -------------------------------------------------------------------- Selecting

/// The result of a successful [`Value::cover`] call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection<'a, T> {
    /// The subset of `utxos` chosen to cover the target, in the order they were picked.
    pub inputs: Vec<&'a T>,
    /// The difference between the combined value of `inputs` and the target; i.e. what would
    /// need to be returned as change.
    pub excess: Value<u64>,
}

/// A single fungible quantity tracked during selection: either lovelace, or a specific native
/// asset identified by its policy (script hash) and asset name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Unit {
    Lovelace,
    Asset(Hash<28>, Vec<u8>),
}

impl Value<u64> {
    /// Attempt to select, from `utxos`, a subset whose combined value covers `target` (i.e. is
    /// greater than or equal to `target` for lovelace and every native asset), trying to keep
    /// the leftover excess (what would need to be returned as change) as small as possible.
    ///
    /// `value_of` extracts the [`Value<u64>`] out of an arbitrary utxo representation `T`, so
    /// this can operate directly over e.g. a slice of full utxo records without requiring the
    /// caller to pre-extract their values.
    ///
    /// Returns `None` when no subset of `utxos` can cover `target` (e.g. there isn't enough of
    /// some asset available in total).
    ///
    /// # Algorithm
    ///
    /// Finding a subset with the *smallest possible* excess is a multi-dimensional variant of
    /// subset-sum, and is NP-hard in general. This method therefore settles for "roughly least
    /// excess" via a greedy heuristic, repeating the following until every requirement is met:
    ///
    /// 1. Identify the *scarcest* outstanding requirement — the lovelace or asset quantity
    ///    satisfied by the fewest remaining (not yet selected) utxos. Ties are broken in favor
    ///    of the requirement with the largest outstanding deficit. This mirrors the common
    ///    coin-selection wisdom of dealing with the hardest-to-satisfy constraint first, so as
    ///    not to paint the selection into a corner.
    /// 2. Among the utxos able to satisfy that requirement, pick the *smallest* one that alone
    ///    covers the remaining deficit, to minimize overshoot; if none can cover it alone, fall
    ///    back to the *largest* one available, to make the most progress towards covering it.
    ///
    /// # examples
    ///
    /// ```rust
    /// # use cardano_sdk::Value;
    /// let utxos = vec![
    ///     Value::<u64>::new(2_000_000),
    ///     Value::<u64>::new(5_000_000),
    ///     Value::<u64>::new(10_000_000),
    /// ];
    ///
    /// let selection = Value::cover(&Value::new(7_000_000), &utxos, |v| v).unwrap();
    ///
    /// assert_eq!(selection.inputs, vec![&Value::new(10_000_000)]);
    /// assert_eq!(selection.excess, Value::new(3_000_000));
    /// ```
    pub fn cover<'a, T>(
        target: &Value<u64>,
        utxos: &'a [T],
        value_of: impl Fn(&T) -> &Value<u64>,
    ) -> Option<Selection<'a, T>> {
        let mut remaining: BTreeMap<Unit, u64> = BTreeMap::new();

        if target.lovelace() > 0 {
            remaining.insert(Unit::Lovelace, target.lovelace());
        }

        for (script_hash, assets) in target.assets() {
            for (asset_name, quantity) in assets {
                if *quantity > 0 {
                    remaining.insert(Unit::Asset(*script_hash, asset_name.clone()), *quantity);
                }
            }
        }

        let mut chosen = vec![false; utxos.len()];
        let mut inputs = Vec::new();
        let mut accumulated = Value::new(0);

        while let Some(unit) = scarcest_unit(&remaining, utxos, &chosen, &value_of) {
            let need = remaining[&unit];

            let candidate = (0..utxos.len())
                .filter(|&i| !chosen[i] && quantity_of(value_of(&utxos[i]), &unit) > 0)
                .min_by_key(|&i| {
                    let have = quantity_of(value_of(&utxos[i]), &unit);
                    if have >= need {
                        (0, have)
                    } else {
                        (1, u64::MAX - have)
                    }
                })?;

            chosen[candidate] = true;
            inputs.push(candidate);

            let value = value_of(&utxos[candidate]);
            accumulated.add(value);

            remaining.retain(|unit, deficit| {
                let contributed = quantity_of(value, unit);
                if contributed >= *deficit {
                    false
                } else {
                    *deficit -= contributed;
                    true
                }
            });
        }

        let excess = excess_of(&accumulated, target)?;

        Some(Selection {
            inputs: inputs.into_iter().map(|i| &utxos[i]).collect(),
            excess,
        })
    }
}

/// Find the outstanding requirement satisfied by the fewest not-yet-selected utxos, breaking
/// ties in favor of the largest remaining deficit. Returns `None` once `remaining` is empty.
fn scarcest_unit<T>(
    remaining: &BTreeMap<Unit, u64>,
    utxos: &[T],
    chosen: &[bool],
    value_of: &impl Fn(&T) -> &Value<u64>,
) -> Option<Unit> {
    let mut best: Option<(Unit, usize, u64)> = None;

    for (unit, deficit) in remaining {
        let candidates = (0..utxos.len())
            .filter(|&i| !chosen[i] && quantity_of(value_of(&utxos[i]), unit) > 0)
            .count();

        let is_better = match &best {
            None => true,
            Some((_, best_candidates, best_deficit)) => {
                candidates < *best_candidates
                    || (candidates == *best_candidates && *deficit > *best_deficit)
            }
        };

        if is_better {
            best = Some((unit.clone(), candidates, *deficit));
        }
    }

    best.map(|(unit, _, _)| unit)
}

/// The quantity of a given [`Unit`] held by a value.
fn quantity_of(value: &Value<u64>, unit: &Unit) -> u64 {
    match unit {
        Unit::Lovelace => value.lovelace(),
        Unit::Asset(script_hash, asset_name) => value
            .assets()
            .get(script_hash)
            .and_then(|assets| assets.get(asset_name))
            .copied()
            .unwrap_or(0),
    }
}

/// Compute `accumulated - target`, returning `None` when `accumulated` does not, in fact, cover
/// `target` for lovelace or some asset.
fn excess_of(accumulated: &Value<u64>, target: &Value<u64>) -> Option<Value<u64>> {
    let lovelace = accumulated.lovelace().checked_sub(target.lovelace())?;

    let mut assets = accumulated.assets().clone();

    for (script_hash, target_assets) in target.assets() {
        for (asset_name, target_quantity) in target_assets {
            if *target_quantity == 0 {
                continue;
            }

            let quantity = assets
                .get_mut(script_hash)
                .and_then(|inner| inner.get_mut(asset_name))?;

            *quantity = quantity.checked_sub(*target_quantity)?;
        }
    }

    Some(Value::new(lovelace).with_assets(assets))
}

// ---------------------------------------------------------------- TESTS

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
    //  ---------------------------------------------------------------- cover

    #[test]
    fn cover_picks_smallest_sufficient_utxo() {
        let utxos = vec![
            Value::<u64>::new(2_000_000),
            Value::<u64>::new(5_000_000),
            Value::<u64>::new(10_000_000),
        ];

        let selection = Value::cover(&Value::new(7_000_000), &utxos, |v| v).unwrap();

        // 10_000_000 is the smallest utxo that alone covers 7_000_000, so it's preferred over
        // combining 2_000_000 + 5_000_000 (which would also work, with less excess, but at the
        // cost of an extra input).
        assert_eq!(selection.inputs, vec![&Value::new(10_000_000)]);
        assert_eq!(selection.excess, Value::new(3_000_000));
    }

    #[test]
    fn cover_exact_match_leaves_no_excess() {
        let utxos = vec![Value::<u64>::new(4_000_000), Value::<u64>::new(6_000_000)];

        let selection = Value::cover(&Value::new(6_000_000), &utxos, |v| v).unwrap();

        assert_eq!(selection.inputs, vec![&Value::new(6_000_000)]);
        assert_eq!(selection.excess, Value::new(0));
    }

    #[test]
    fn cover_combines_multiple_utxos_when_none_is_sufficient_alone() {
        let utxos = vec![
            Value::<u64>::new(2_000_000),
            Value::<u64>::new(3_000_000),
            Value::<u64>::new(3_500_000),
        ];

        let selection = Value::cover(&Value::new(8_000_000), &utxos, |v| v).unwrap();

        let mut total = Value::new(0);
        for input in &selection.inputs {
            total.add(input);
        }

        assert_eq!(total.lovelace(), 8_500_000);
        assert_eq!(selection.excess, Value::new(500_000));
    }

    #[test]
    fn cover_returns_none_when_lovelace_is_insufficient() {
        let utxos = vec![Value::<u64>::new(1_000_000), Value::<u64>::new(2_000_000)];

        assert!(Value::cover(&Value::new(10_000_000), &utxos, |v| v).is_none());
    }

    #[test]
    fn cover_returns_none_when_required_asset_is_absent() {
        let target: Value<u64> = value!(
            2_000_000,
            (
                "279c909f348e533da5808898f87f9a14bb2c3dfbbacccd631d927a3f",
                "534e454b",
                100
            ),
        );

        let utxos = vec![Value::<u64>::new(10_000_000)];

        assert!(Value::cover(&target, &utxos, |v| v).is_none());
    }

    #[test]
    fn cover_selects_the_utxo_holding_the_required_asset() {
        let target: Value<u64> = value!(
            2_000_000,
            (
                "279c909f348e533da5808898f87f9a14bb2c3dfbbacccd631d927a3f",
                "534e454b",
                100
            ),
        );

        let with_asset: Value<u64> = value!(
            1_500_000,
            (
                "279c909f348e533da5808898f87f9a14bb2c3dfbbacccd631d927a3f",
                "534e454b",
                250
            ),
        );
        let without_asset = Value::<u64>::new(5_000_000);

        let utxos = vec![without_asset.clone(), with_asset.clone()];

        let selection = Value::cover(&target, &utxos, |v| v).unwrap();

        assert!(selection.inputs.contains(&&with_asset));
    }

    #[test]
    fn cover_combines_utxos_across_assets_and_lovelace() {
        let target: Value<u64> = value!(
            9_000_000,
            (
                "279c909f348e533da5808898f87f9a14bb2c3dfbbacccd631d927a3f",
                "534e454b",
                100
            ),
            (
                "a0028f350aaabe0545fdcb56b039bfb08e4bb4d8c4d7c3c7d481c235",
                "484f534b59",
                1000
            ),
        );

        // Each of these carries just one of the two required assets, plus some lovelace that,
        // combined, still falls short of the target.
        let snek_utxo: Value<u64> = value!(
            2_000_000,
            (
                "279c909f348e533da5808898f87f9a14bb2c3dfbbacccd631d927a3f",
                "534e454b",
                150
            ),
        );
        let hosky_utxo: Value<u64> = value!(
            2_000_000,
            (
                "a0028f350aaabe0545fdcb56b039bfb08e4bb4d8c4d7c3c7d481c235",
                "484f534b59",
                1200
            ),
        );
        let ada_only = Value::<u64>::new(6_000_000);

        let utxos = vec![snek_utxo.clone(), hosky_utxo.clone(), ada_only.clone()];

        let selection = Value::cover(&target, &utxos, |v| v).unwrap();

        // All three utxos are needed: one for each asset, and the third to make up the
        // remaining lovelace deficit.
        assert!(selection.inputs.contains(&&snek_utxo));
        assert!(selection.inputs.contains(&&hosky_utxo));
        assert!(selection.inputs.contains(&&ada_only));
        assert_eq!(selection.inputs.len(), 3);

        let mut total = Value::new(0);
        for input in &selection.inputs {
            total.add(input);
        }
        assert_eq!(
            total.lovelace() - target.lovelace(),
            selection.excess.lovelace()
        );
    }

    #[test]
    fn cover_excess_includes_leftover_of_an_unrequested_asset() {
        // The only utxo available happens to carry an asset that the target doesn't ask for at
        // all; since it's the only way to reach the lovelace target, it gets selected, and the
        // whole of that asset ends up as excess.
        let target = Value::<u64>::new(1_000_000);

        let utxo: Value<u64> = value!(
            2_000_000,
            (
                "279c909f348e533da5808898f87f9a14bb2c3dfbbacccd631d927a3f",
                "534e454b",
                42
            ),
        );

        let utxos = vec![utxo.clone()];

        let selection = Value::cover(&target, &utxos, |v| v).unwrap();

        let expected_excess: Value<u64> = value!(
            1_000_000,
            (
                "279c909f348e533da5808898f87f9a14bb2c3dfbbacccd631d927a3f",
                "534e454b",
                42
            ),
        );

        assert_eq!(selection.inputs, vec![&utxo]);
        assert_eq!(selection.excess, expected_excess);
    }
}
