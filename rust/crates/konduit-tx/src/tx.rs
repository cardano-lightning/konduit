use std::{cmp, collections::BTreeMap};

use cardano_sdk::{
    Address, ChangeStrategy, PlutusData, SlotBound, Transaction, address::kind,
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
    let gain = steppeds.gain() - opens.iter().map(|x| x.buffered_amount()).sum::<u64>() as i64;
    let fuel_amount = cmp::max(FEE_BUFFER, FEE_BUFFER.saturating_sub_signed(gain));
    let fuel_inputs = fuel::select(fuel, fuel_amount)?;
    let inputs = steppeds
        .inputs()
        .iter()
        .map(|i| (i.0.clone(), Some(PlutusData::from(i.1.clone()))))
        .chain(fuel_inputs.iter().map(|i| (i.clone(), None)))
        .collect::<Vec<_>>();
    let outputs: Vec<_> = steppeds
        .outputs()
        .into_iter()
        .chain(opens.iter().map(|o| o.output(network_id)))
        .collect();
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

    let utxos = steppeds
        .utxos()
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
