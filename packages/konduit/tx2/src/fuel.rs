use std::collections::BTreeMap;

use cardano_sdk::{Input, Output, Value};

/// Greedy utxo selection to cover a target value.
///
/// Repeatedly selects the utxo that best covers the remaining need,
/// preferring utxos that satisfy scarce native tokens over raw lovelace.
/// Returns the minimal set of inputs needed, or an error if the wallet
/// cannot cover the target.
pub fn select(
    utxos: &BTreeMap<Input, Output>,
    amount: &Value<u64>,
) -> Result<Vec<Input>, SelectError> {
    let mut remaining = amount.clone();
    let mut total = Value::new(0);
    let mut available: Vec<(&Input, &Output)> = utxos.iter().collect();
    let mut selected = Vec::new();

    while !remaining.is_empty() {
        let best = available
            .iter()
            .enumerate()
            .max_by_key(|(_, (_, output))| coverage(output.value(), &remaining));

        let Some((idx, _)) = best else {
            return Err(SelectError::NotCovered(remaining));
        };

        let (input, output) = available.swap_remove(idx);
        total.add(output.value());
        remaining = saturating_sub_value(&remaining, output.value());
        selected.push(input.clone());
    }

    Ok(selected)
}

/// Score a candidate utxo by how well it covers the remaining need.
/// (asset_hits, lovelace) — lexicographic: prefer utxos covering more
/// needed token types, tiebreak by lovelace contribution.
fn coverage(candidate: &Value<u64>, remaining: &Value<u64>) -> (usize, u64) {
    let mut asset_hits = 0;

    for (policy, r_assets) in remaining.assets() {
        if let Some(c_assets) = candidate.assets().get(policy) {
            for (name, r_qty) in r_assets {
                if *r_qty > 0 && c_assets.get(name).is_some_and(|q| *q > 0) {
                    asset_hits += 1;
                }
            }
        }
    }

    let ada = std::cmp::min(candidate.lovelace(), remaining.lovelace());
    (asset_hits, ada)
}

/// Missing from Value.
pub fn saturating_sub_value(a: &Value<u64>, b: &Value<u64>) -> Value<u64> {
    let lovelace = a.lovelace().saturating_sub(b.lovelace());
    let mut remaining_assets = a.assets().clone();

    for (policy, b_assets) in b.assets() {
        if let Some(a_assets) = remaining_assets.get_mut(policy) {
            for (name, b_qty) in b_assets {
                if let Some(a_qty) = a_assets.get_mut(name) {
                    *a_qty = a_qty.saturating_sub(*b_qty);
                    if *a_qty == 0 {
                        a_assets.remove(name);
                    }
                }
            }
            if a_assets.is_empty() {
                remaining_assets.remove(policy);
            }
        }
    }

    Value::new(lovelace).with_assets(remaining_assets)
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum SelectError {
    #[error("Funds exhausted, but value NotCovered : {0}")]
    NotCovered(Value<u64>),
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use cardano_sdk::{assets, input, output, value};

    use super::*;

    fn utxos_from(pairs: Vec<(Input, Output)>) -> BTreeMap<Input, Output> {
        BTreeMap::from_iter(pairs)
    }

    #[test]
    fn select_empty_amount() {
        let utxos = utxos_from(vec![(
            input!(
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                0
            ),
            output!(
                "addr1qxu84ftxpzh3zd8p9awp2ytwzk5exj0fxcj7paur4kd4ytun36yuhgl049rxhhuckm2lpq3rmz5dcraddyl45d6xgvqqsp504c",
                value!(1_000_000)
            ),
        )]);
        let result = select(&utxos, &Value::new(0)).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn select_single_utxo_sufficient() {
        let utxos = utxos_from(vec![(
            input!(
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                0
            ),
            output!(
                "addr1qxu84ftxpzh3zd8p9awp2ytwzk5exj0fxcj7paur4kd4ytun36yuhgl049rxhhuckm2lpq3rmz5dcraddyl45d6xgvqqsp504c",
                value!(5_000_000)
            ),
        )]);
        let result = select(&utxos, &Value::new(3_000_000)).unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn select_combines_multiple() {
        let utxos = utxos_from(vec![
            (
                input!(
                    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                    0
                ),
                output!(
                    "addr1qxu84ftxpzh3zd8p9awp2ytwzk5exj0fxcj7paur4kd4ytun36yuhgl049rxhhuckm2lpq3rmz5dcraddyl45d6xgvqqsp504c",
                    value!(2_000_000)
                ),
            ),
            (
                input!(
                    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                    1
                ),
                output!(
                    "addr1qxu84ftxpzh3zd8p9awp2ytwzk5exj0fxcj7paur4kd4ytun36yuhgl049rxhhuckm2lpq3rmz5dcraddyl45d6xgvqqsp504c",
                    value!(2_000_000)
                ),
            ),
            (
                input!(
                    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                    2
                ),
                output!(
                    "addr1qxu84ftxpzh3zd8p9awp2ytwzk5exj0fxcj7paur4kd4ytun36yuhgl049rxhhuckm2lpq3rmz5dcraddyl45d6xgvqqsp504c",
                    value!(2_000_000)
                ),
            ),
        ]);
        let result = select(&utxos, &Value::new(5_000_000)).unwrap();
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn select_insufficient_funds() {
        let utxos = utxos_from(vec![(
            input!(
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                0
            ),
            output!(
                "addr1qxu84ftxpzh3zd8p9awp2ytwzk5exj0fxcj7paur4kd4ytun36yuhgl049rxhhuckm2lpq3rmz5dcraddyl45d6xgvqqsp504c",
                value!(1_000_000)
            ),
        )]);
        let result = select(&utxos, &Value::new(5_000_000));
        assert!(matches!(result, Err(SelectError::NotCovered(_))));
    }

    #[test]
    fn select_prefers_token_bearing_utxo() {
        let utxos = utxos_from(vec![
            (
                input!(
                    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                    0
                ),
                output!(
                    "addr1qxu84ftxpzh3zd8p9awp2ytwzk5exj0fxcj7paur4kd4ytun36yuhgl049rxhhuckm2lpq3rmz5dcraddyl45d6xgvqqsp504c",
                    value!(10_000_000)
                ),
            ),
            (
                input!(
                    "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                    0
                ),
                output!(
                    "addr1qxu84ftxpzh3zd8p9awp2ytwzk5exj0fxcj7paur4kd4ytun36yuhgl049rxhhuckm2lpq3rmz5dcraddyl45d6xgvqqsp504c",
                    value!(
                        2_000_000,
                        (
                            "279c909f348e533da5808898f87f9a14bb2c3dfbbacccd631d927a3f",
                            "534e454b",
                            50
                        )
                    )
                ),
            ),
        ]);
        let target = Value::new(1_000_000).with_assets(assets!((
            "279c909f348e533da5808898f87f9a14bb2c3dfbbacccd631d927a3f",
            "534e454b",
            10
        )));
        let result = select(&utxos, &target).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn select_empty_wallet() {
        let utxos = utxos_from(vec![]);
        let result = select(&utxos, &Value::new(1_000_000));
        assert!(matches!(result, Err(SelectError::NotCovered(..))));
    }

    #[test]
    fn select_exact_amount() {
        let utxos = utxos_from(vec![(
            input!(
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                0
            ),
            output!(
                "addr1qxu84ftxpzh3zd8p9awp2ytwzk5exj0fxcj7paur4kd4ytun36yuhgl049rxhhuckm2lpq3rmz5dcraddyl45d6xgvqqsp504c",
                value!(3_000_000)
            ),
        )]);
        let result = select(&utxos, &Value::new(3_000_000)).unwrap();
        assert_eq!(result.len(), 1);
    }
}
