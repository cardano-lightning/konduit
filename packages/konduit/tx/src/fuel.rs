use cardano_sdk::{Input, Value};

use crate::{Lovelace, Utxos};

/// Select wallet UTxOs covering every unit in the target value.
pub fn select(utxos: &Utxos, target: &Value<u64>) -> anyhow::Result<Vec<Input>> {
    if target.is_empty() {
        return Ok(vec![]);
    }
    let candidates = utxos.iter().collect::<Vec<_>>();
    let selection = Value::cover(target, &candidates, |(_, output)| output.value())
        .ok_or_else(|| anyhow::anyhow!("insufficient wallet value to cover target {target}"))?;
    Ok(selection
        .inputs
        .into_iter()
        .map(|(input, _)| (*input).clone())
        .collect())
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
}
