use cardano_sdk::{Input, Value};

use crate::{Lovelace, Utxos};

pub fn select(utxos: &Utxos, target: &Value<u64>) -> anyhow::Result<Vec<Input>> {
    if target.is_empty() {
        return Ok(vec![]);
    }
    let candidates = utxos.iter().collect::<Vec<_>>();
    let clean: Vec<_> = candidates
        .iter()
        .copied()
        .filter(|(_, output)| !has_unrelated_tokens(output.value(), target))
        .collect();
    let selection = Value::cover(target, &clean, |(_, output)| output.value())
        .or_else(|| Value::cover(target, &candidates, |(_, output)| output.value()))
        .ok_or_else(|| anyhow::anyhow!("insufficient wallet value to cover target {target}"))?;
    Ok(selection
        .inputs
        .into_iter()
        .map(|(input, _)| (*input).clone())
        .collect())
}

pub fn select_collateral(utxos: &Utxos, minimum: Lovelace) -> anyhow::Result<Input> {
    utxos
        .iter()
        .filter(|(_, output)| {
            output.script().is_none()
                && output.value().assets().is_empty()
                && output.value().lovelace() >= minimum
                && output
                    .address()
                    .as_shelley()
                    .is_some_and(|address| address.payment().as_key().is_some())
        })
        .min_by_key(|(_, output)| output.value().lovelace())
        .map(|(input, _)| input.clone())
        .ok_or_else(|| anyhow::anyhow!("no Ada-only collateral UTxO covers {minimum} lovelace"))
}

fn has_unrelated_tokens(value: &Value<u64>, target: &Value<u64>) -> bool {
    value.assets().iter().any(|(policy, names)| {
        names.iter().any(|(name, quantity)| {
            *quantity > 0
                && target
                    .assets()
                    .get(policy)
                    .and_then(|names| names.get(name))
                    .is_none()
        })
    })
}

/// Select utxos to cover fees and collaterals
pub fn select_no_script(utxos: &Utxos, amount: Lovelace) -> anyhow::Result<Vec<Input>> {
    let utxos = utxos
        .iter()
        .filter(|(_, output)| output.script().is_none())
        .filter(|(_, output)| {
            output
                .address()
                .as_shelley()
                .is_some_and(|addr| addr.payment().as_key().is_some())
        })
        .map(|x| (x.0.clone(), x.1.clone())) // FIXME :: How ought we avoid clone?
        .collect();
    select(&utxos, &Value::new(amount))
}

#[cfg(test)]
mod tests {
    use cardano_sdk::{Address, Hash, Output};

    use super::*;
    use crate::tx::FEE_BUFFER;

    #[test]
    fn token_target_selects_token_utxo() {
        let policy = Hash::<28>::new([1; 28]);
        let ada_input = Input::new(Hash::<32>::new([1; 32]), 0);
        let token_input = Input::new(Hash::<32>::new([2; 32]), 0);
        let utxos = Utxos::from([
            (
                ada_input,
                Output::new(Address::default(), Value::new(5_000_000)),
            ),
            (
                token_input.clone(),
                Output::new(
                    Address::default(),
                    Value::new(2_000_000).with_assets([(policy, [(b"TOKEN", 10)])]),
                ),
            ),
        ]);
        let target = Value::new(3_000_000).with_assets([(policy, [(b"TOKEN", 5)])]);

        let selected = select(&utxos, &target).unwrap();
        assert!(selected.contains(&token_input));

        let lovelace_only = Utxos::from([(
            Input::new(Hash::<32>::new([3; 32]), 0),
            Output::new(Address::default(), Value::new(10_000_000)),
        )]);
        assert!(select(&lovelace_only, &target).is_err());
    }

    #[test]
    fn collateral_is_one_ada_only_key_output() {
        let address =
            cardano_sdk::VerificationKey::from([9; 32]).to_address(cardano_sdk::NetworkId::TESTNET);
        let small = Input::new(Hash::<32>::new([4; 32]), 0);
        let selected = Input::new(Hash::<32>::new([5; 32]), 0);
        let token = Input::new(Hash::<32>::new([6; 32]), 0);
        let policy = Hash::<28>::new([1; 28]);
        let utxos = Utxos::from([
            (
                small,
                Output::new(address.clone().into(), Value::new(FEE_BUFFER - 1)),
            ),
            (
                selected.clone(),
                Output::new(address.clone().into(), Value::new(FEE_BUFFER)),
            ),
            (
                token,
                Output::new(
                    address.into(),
                    Value::new(FEE_BUFFER + 1).with_assets([(policy, [(b"T", 1)])]),
                ),
            ),
        ]);
        assert_eq!(select_collateral(&utxos, FEE_BUFFER).unwrap(), selected);
    }
}
