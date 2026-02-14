use std::{collections::BTreeMap, iter};

use cardano_tx_builder::{
    Address, ChangeStrategy, Credential, Hash, Input, PlutusData, SlotBound, Transaction,
    VerificationKey, transaction::state::ReadyForSigning,
};
use konduit_data::{Cont, Duration, Keytag, Receipt, Redeemer, Step};

use crate::{
    ChannelOutput, NetworkParameters, Utxos, extract_amount, filter_channels, konduit_reference,
    wallet_inputs,
};

#[derive(Debug, Clone)]
pub struct AdaptorPreferences {
    // Prevents spending a utxo that would result in too little gain relative to the cost of inclusion.
    pub min_single: u64,
    // Prevents a transaction in which the total gain is too little
    pub min_total: u64,
}

// WARNING :: This transaction does **not** verify that the resultant tx does not
// violate the condition that if the channel is being treated as active,
// then the retainer is not responded.
// This must be handled elsewhere!
pub fn tx(
    network_parameters: &NetworkParameters,
    preferences: &AdaptorPreferences,
    wallet: &VerificationKey,
    receipts: &BTreeMap<Keytag, Receipt>,
    utxos: &Utxos,
    upper_bound: &Duration,
) -> anyhow::Result<Transaction<ReadyForSigning>> {
    let Some(reference_input) = konduit_reference(utxos) else {
        return Err(anyhow::anyhow!("No konduit reference found"));
    };
    let reference_inputs = vec![reference_input];
    let wallet_ins = wallet_inputs(wallet, utxos);
    let channels_in = filter_channels(utxos, |c| c.constants.sub_vkey == *wallet);

    let mk_step_ = |c: &ChannelOutput| mk_step(upper_bound, receipts, c);
    let mut steps = channels_in
        .iter()
        .filter_map(|(i, c)| mk_step_(c).map(|(step, output)| (i.clone(), step, output)))
        .filter(|triple| {
            extract_amount(utxos.get(&triple.0).unwrap().value()) - triple.2.amount
                >= preferences.min_single
        })
        .collect::<Vec<_>>();

    if steps
        .iter()
        .map(|triple| extract_amount(utxos.get(&triple.0).unwrap().value()) - triple.2.amount)
        .sum::<u64>()
        < preferences.min_total
    {
        return Err(anyhow::anyhow!("Insufficient total gain"));
    }
    steps.sort_by_key(|(i, _, _)| i.clone());
    let [main_step, rest @ ..] = &steps[..] else {
        panic!("Impossible")
    };
    let main_redeemer = Redeemer::Main(
        iter::once(main_step.1.clone())
            .chain(rest.iter().map(|s| s.1.clone()))
            .map(|s| Step::Cont(s.clone()))
            .collect::<Vec<_>>(),
    );
    let main_input = (main_step.0.clone(), Some(PlutusData::from(main_redeemer)));
    let defer_inputs = rest
        .iter()
        .map(|(i, _, _)| (i.clone(), Some(PlutusData::from(Redeemer::Defer)).clone()))
        .collect::<Vec<(Input, Option<PlutusData>)>>();
    let outputs = steps
        .iter()
        .map(|(i, _, co)| {
            co.to_output(
                &network_parameters.network_id,
                &utxos
                    .get(i)
                    .unwrap()
                    .address()
                    .as_shelley()
                    .unwrap()
                    .delegation(),
            )
        })
        .collect::<Vec<_>>();
    let wallet_hash = Hash::<28>::new(wallet);
    let specified_signatories = vec![wallet_hash];
    let inputs = wallet_ins
        .iter()
        .map(|i| (i.clone(), None))
        .chain(iter::once(main_input))
        .chain(defer_inputs)
        .collect::<Vec<_>>();

    // FIXME :: This bounds should not _necessarily_ be necessary.
    let upper_bound = SlotBound::Exclusive(
        network_parameters
            .protocol_parameters
            .posix_to_slot(upper_bound.0),
    );

    Transaction::build(
        &network_parameters.protocol_parameters,
        utxos,
        |transaction| {
            let wallet_address = Address::new(
                network_parameters.network_id,
                Credential::from_key(wallet_hash),
            );
            transaction
                .with_inputs(inputs.clone())
                .with_collaterals(wallet_ins.clone())
                .with_reference_inputs(reference_inputs.clone())
                .with_outputs(outputs.clone())
                .with_specified_signatories(specified_signatories.clone())
                .with_validity_interval(SlotBound::None, upper_bound)
                .with_change_strategy(ChangeStrategy::as_last_output(wallet_address.into()))
                .ok()
        },
    )
}

fn mk_step(
    upper_bound: &Duration,
    receipts: &BTreeMap<Keytag, Receipt>,
    c: &ChannelOutput,
) -> Option<(Cont, ChannelOutput)> {
    receipts
        .get(&Keytag::new(c.constants.add_vkey, c.constants.tag.clone()))
        .and_then(|receipt| receipt.step(upper_bound, &c.to_l1_channel()))
        .map(|(s, l1)| (s, ChannelOutput::from_l1_channel(l1, c.constants.clone())))
}
