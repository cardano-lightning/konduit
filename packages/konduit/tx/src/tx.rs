use std::{cmp, collections::BTreeMap};

use cardano_sdk::{
    Address, ChangeStrategy, Hash, PlutusData, SlotBound, Transaction, Value, address::kind,
    transaction::state::ReadyForSigning,
};
use konduit_data::Duration;

use crate::{Lovelace, NetworkParameters, Open, SteppedUtxos, Utxo, Utxos, fuel};

pub const FEE_BUFFER: Lovelace = 3_000_000;

pub fn tx(
    network_parameters: &NetworkParameters,
    reference_utxo: Option<&Utxo>,
    change_address: Address<kind::Any>,
    steppeds: SteppedUtxos,
    opens: Vec<Open>,
    fuel: &Utxos,
) -> anyhow::Result<Transaction<ReadyForSigning>> {
    let network_id = network_parameters.network_id;
    let reference_inputs: Vec<_> = reference_utxo.iter().map(|x| x.0.clone()).collect();
    if !steppeds.inputs().is_empty() && reference_inputs.is_empty() {
        return Err(anyhow::anyhow!(
            "Reference script required when stepping channels"
        ));
    }
    let outputs: Vec<_> = steppeds
        .outputs()
        .into_iter()
        .chain(opens.iter().map(|o| o.output(network_id)))
        .collect();
    let stepped_utxos = steppeds.utxos();
    let target = wallet_target(
        stepped_utxos.values().map(|output| output.value()),
        outputs.iter().map(|output| output.value()),
    )?;
    let fuel_inputs = fuel::select(fuel, &target)?;
    let inputs = steppeds
        .inputs()
        .iter()
        .map(|i| (i.0.clone(), Some(PlutusData::from(i.1.clone()))))
        .chain(fuel_inputs.iter().map(|i| (i.clone(), None)))
        .collect::<Vec<_>>();
    let outputs = outputs;
    let collaterals = fuel_inputs.clone();
    let specified_signatories = steppeds.specified_signatories();
    let bounds = steppeds.bounds();

    let to_slot = |d: Duration| network_parameters.protocol_parameters.posix_to_slot(*d);

    let lower_bound = bounds
        .lower
        .map_or(SlotBound::None, |d| SlotBound::Inclusive(to_slot(d)));
    let upper_bound = bounds
        .upper
        .map_or(SlotBound::None, |d| SlotBound::Exclusive(to_slot(d)));

    let utxos = stepped_utxos
        .iter()
        .chain(fuel.iter())
        .map(|t| (t.0.clone(), t.1.clone()))
        .chain(reference_utxo.iter().map(|i| (i.0.clone(), i.1.clone())))
        .collect::<BTreeMap<_, _>>();
    Transaction::build(
        &network_parameters.protocol_parameters,
        &utxos,
        |transaction| {
            transaction
                .with_inputs(inputs.clone())
                .with_collaterals(collaterals.clone())
                .with_reference_inputs(reference_inputs.clone())
                .with_outputs(outputs.clone())
                .with_specified_signatories(specified_signatories.clone())
                .with_validity_interval(lower_bound, upper_bound)
                .with_change_strategy(ChangeStrategy::as_last_output(change_address.clone()))
                .ok()
        },
    )
}

fn wallet_target<'a>(
    inputs: impl Iterator<Item = &'a Value<u64>>,
    outputs: impl Iterator<Item = &'a Value<u64>>,
) -> anyhow::Result<Value<u64>> {
    let mut lovelace = 0_i128;
    let mut assets = BTreeMap::<(Hash<28>, Vec<u8>), i128>::new();
    for value in inputs {
        accumulate(&mut lovelace, &mut assets, value, -1)?;
    }
    for value in outputs {
        accumulate(&mut lovelace, &mut assets, value, 1)?;
    }

    let lovelace = cmp::max(
        i128::from(FEE_BUFFER),
        i128::from(FEE_BUFFER)
            .checked_add(lovelace)
            .ok_or_else(|| anyhow::anyhow!("wallet target lovelace overflow"))?,
    );
    let lovelace =
        u64::try_from(lovelace).map_err(|_| anyhow::anyhow!("wallet target lovelace overflow"))?;
    let mut target_assets = BTreeMap::<Hash<28>, BTreeMap<Vec<u8>, u64>>::new();
    for ((policy, name), quantity) in assets {
        if quantity > 0 {
            target_assets.entry(policy).or_default().insert(
                name,
                u64::try_from(quantity)
                    .map_err(|_| anyhow::anyhow!("wallet target asset overflow"))?,
            );
        }
    }
    Ok(Value::new(lovelace).with_assets(target_assets))
}

fn accumulate(
    lovelace: &mut i128,
    assets: &mut BTreeMap<(Hash<28>, Vec<u8>), i128>,
    value: &Value<u64>,
    sign: i128,
) -> anyhow::Result<()> {
    *lovelace = lovelace
        .checked_add(i128::from(value.lovelace()) * sign)
        .ok_or_else(|| anyhow::anyhow!("transaction lovelace total overflow"))?;
    for (policy, names) in value.assets() {
        for (name, quantity) in names {
            let total = assets.entry((*policy, name.clone())).or_default();
            *total = total
                .checked_add(i128::from(*quantity) * sign)
                .ok_or_else(|| anyhow::anyhow!("transaction asset total overflow"))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wallet_target_covers_only_value_deficits() {
        let policy = Hash::<28>::new([1; 28]);
        let input = Value::new(2_000_000).with_assets([(policy, [(b"TOKEN", 20)])]);
        let output = Value::new(2_000_000).with_assets([(policy, [(b"TOKEN", 25)])]);
        let target = wallet_target([&input].into_iter(), [&output].into_iter()).unwrap();
        assert_eq!(target.lovelace(), FEE_BUFFER);
        assert_eq!(target.assets()[&policy][b"TOKEN".as_slice()], 5);

        let open = Value::new(2_000_000).with_assets([(policy, [(b"TOKEN", 25)])]);
        let target = wallet_target([].into_iter(), [&open].into_iter()).unwrap();
        assert_eq!(target.lovelace(), FEE_BUFFER + 2_000_000);
        assert_eq!(target.assets()[&policy][b"TOKEN".as_slice()], 25);
    }
}
