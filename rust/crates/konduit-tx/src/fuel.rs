use cardano_sdk::{Input, Output};

use crate::{Lovelace, Utxos};

/// Select utxos to cover fees and collaterals
pub fn select(utxos: &Utxos, amount: Lovelace) -> anyhow::Result<Vec<Input>> {
    if amount == 0 {
        return Ok(vec![]);
    }
    let mut sorted_utxos: Vec<(&Input, &Output)> = utxos.iter().collect();
    sorted_utxos.sort_by_key(|(_, output)| std::cmp::Reverse(output.value().lovelace()));

    let mut selected_inputs = Vec::new();
    let mut total_lovelace: u64 = 0;

    for (input, output) in sorted_utxos {
        selected_inputs.push(input.clone());
        total_lovelace = total_lovelace.saturating_add(output.value().lovelace());

        if total_lovelace >= amount {
            break;
        }
    }

    if total_lovelace < amount {
        return Err(anyhow::anyhow!(
            "insufficient funds in wallet to cover the amount"
        ));
    }

    Ok(selected_inputs)
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
    select(&utxos, amount)
}
