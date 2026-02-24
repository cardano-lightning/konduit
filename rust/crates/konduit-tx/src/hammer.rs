use crate::{
    Bounds, ChannelOutput, KONDUIT_VALIDATOR, MIN_ADA_BUFFER, NetworkParameters, Utxos,
    filter_channels, konduit_reference, wallet_inputs,
};
use anyhow::anyhow;
use cardano_sdk::{
    Address, ChangeStrategy, Credential, Hash, Input, Output, PlutusData, SlotBound, Transaction,
    Value, VerificationKey, transaction::state::ReadyForSigning,
};
use konduit_data::{Constants, Cont, Duration, Eol, Keytag, Redeemer, Stage, Step, Unpend};
use std::{
    collections::{BTreeMap, HashSet},
    iter,
};

#[derive(Debug)]
pub struct OpenIntent {
    pub constants: Constants,
    pub amount: u64,
}

#[derive(Debug)]
pub enum Intent {
    Add(u64),
    Close,
}

pub fn tx(
    network_parameters: &NetworkParameters,
    wallet: &VerificationKey,
    opens: &Vec<OpenIntent>,
    intents: &BTreeMap<Keytag, Intent>,
    utxos: &Utxos,
    bounds: Bounds,
) -> anyhow::Result<Transaction<ReadyForSigning>> {
    let reference_input = konduit_reference(utxos);
    let reference_inputs = match reference_input {
        None => {
            if !intents.is_empty() {
                return Err(anyhow!(
                    "No reference script found. Cannot close without reference script"
                ));
            }
            if opens.is_empty() {
                return Err(anyhow!(
                    "No reference script found. Can only open but none given"
                ));
            }
            vec![]
        }
        Some(reference_input) => vec![reference_input],
    };
    let wallet_ins = wallet_inputs(wallet, utxos);
    let channels_in = filter_channels(utxos, |_| true);
    // FIXME :: support delegation
    let konduit_address = Address::from(Address::new(
        network_parameters.network_id,
        Credential::from_script(KONDUIT_VALIDATOR.hash),
    ));

    let mk_step_ = |c: &ChannelOutput| mk_step(&bounds, &intents, c);

    let mut steps = channels_in
        .iter()
        .filter_map(|(i, c)| mk_step_(c).map(|step_to| (i.clone(), step_to)))
        .collect::<Vec<(Input, StepTo)>>();
    steps.sort_by_key(|(i, _c)| i.clone());

    for step in &steps {
        println!("{:#}", step.0);
    }

    let channel_inputs = match &steps[..] {
        [main_step, defers @ ..] => {
            let main_redeemer = Redeemer::Main(
                iter::once(main_step.1.to_step())
                    .chain(defers.iter().map(|(_, s)| s.to_step()))
                    .collect::<Vec<_>>(),
            );
            iter::once((main_step.0.clone(), Some(PlutusData::from(main_redeemer))))
                .chain(
                    defers
                        .iter()
                        .map(|(i, _)| (i.clone(), Some(PlutusData::from(Redeemer::Defer)).clone())),
                )
                .collect::<Vec<(Input, Option<PlutusData>)>>()
        }
        _ => vec![],
    };
    let opens = opens.iter().map(|o| {
        Output::new(
            konduit_address.clone(),
            Value::new(MIN_ADA_BUFFER + o.amount),
        )
        .with_datum(PlutusData::from(konduit_data::Datum {
            own_hash: KONDUIT_VALIDATOR.hash,
            constants: o.constants.clone(),
            stage: Stage::Opened(0, vec![]),
        }))
    });

    if channel_inputs.is_empty() && opens.len() == 0 {
        return Err(anyhow::anyhow!("Transaction does nothing!"));
    }

    let outputs = steps
        .iter()
        .filter_map(|(i, s)| {
            s.to_output().map(|co| {
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
        })
        .chain(opens)
        .collect::<Vec<_>>();

    for step in &steps {
        println!("{:?}", step.1);
    }
    for output in &outputs {
        println!("{:#}", output);
    }
    let wallet_hash = Hash::<28>::new(wallet);
    let specified_signatories = channels_in
        .iter()
        .map(|channel| Hash::<28>::new(channel.1.constants.add_vkey.clone()))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let inputs = wallet_ins
        .iter()
        .map(|i| (i.clone(), None))
        .chain(channel_inputs)
        .collect::<Vec<_>>();

    // FIXME :: These bounds should not be necessary,
    // instead only used when a step requires.
    let lower_bound = SlotBound::Inclusive(
        network_parameters
            .protocol_parameters
            .posix_to_slot(*bounds.lower),
    );

    let upper_bound = SlotBound::Exclusive(
        network_parameters
            .protocol_parameters
            .posix_to_slot(*bounds.upper),
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
                .with_validity_interval(lower_bound, upper_bound)
                .with_change_strategy(ChangeStrategy::as_last_output(wallet_address.into()))
                .ok()
        },
    )
}

#[derive(Debug)]
pub enum StepTo {
    Cont(Cont, Box<ChannelOutput>),
    Eol(Eol),
}

impl StepTo {
    pub fn to_step(&self) -> Step {
        match &self {
            StepTo::Cont(cont, _) => Step::Cont(cont.clone()),
            StepTo::Eol(eol) => Step::Eol(eol.clone()),
        }
    }
    pub fn to_output(&self) -> Option<ChannelOutput> {
        match &self {
            StepTo::Cont(_, o) => Some(o.as_ref().clone()),
            StepTo::Eol(_) => None,
        }
    }
}

fn mk_step(
    bounds: &Bounds,
    intents: &BTreeMap<Keytag, Intent>,
    c: &ChannelOutput,
) -> Option<StepTo> {
    match &c.stage {
        Stage::Opened(subbed, useds) => {
            match intents.get(&c.keytag())? {
                Intent::Add(add) => Some(StepTo::Cont(
                    Cont::Add,
                    Box::new(ChannelOutput {
                        amount: add + c.amount,
                        constants: c.constants.clone(),
                        stage: Stage::Opened(*subbed, useds.clone()),
                    }),
                )),
                Intent::Close => {
                    // FIXME :: This coersion should not be necessary. Upstream a fix
                    let elapse_at = Duration::from_millis(
                        bounds.upper.as_millis() as u64
                            + c.constants.close_period.as_millis() as u64,
                    );
                    Some(StepTo::Cont(
                        Cont::Close,
                        Box::new(ChannelOutput {
                            amount: c.amount,
                            constants: c.constants.clone(),
                            stage: Stage::Closed(*subbed, useds.clone(), elapse_at),
                        }),
                    ))
                }
            }
        }
        Stage::Closed(_, _, elapse_at) => {
            if elapse_at.as_millis() < bounds.lower.as_millis() {
                Some(StepTo::Eol(Eol::Elapse))
            } else {
                None
            }
        }
        Stage::Responded(pendings_amount, pendings) => {
            let unpends = pendings
                .iter()
                .map(|p| {
                    if p.timeout.as_millis() < bounds.lower.as_millis() {
                        Unpend::Expire
                    } else {
                        Unpend::Continue
                    }
                })
                .collect::<Vec<_>>();
            let claimable = c.amount - pendings_amount
                + pendings
                    .iter()
                    .zip(unpends.iter())
                    .filter(|(_a, b)| matches!(b, Unpend::Expire))
                    .map(|(a, _b)| a.amount)
                    .sum::<u64>();
            if claimable > 0 {
                let cont_pendings = pendings
                    .iter()
                    .zip(unpends.iter())
                    .filter(|(_a, b)| matches!(b, Unpend::Continue))
                    .map(|(a, _b)| a.clone())
                    .collect::<Vec<_>>();
                if cont_pendings.is_empty() {
                    Some(StepTo::Eol(Eol::End))
                } else {
                    Some(StepTo::Cont(
                        Cont::Expire(unpends),
                        Box::new(ChannelOutput {
                            amount: c.amount - claimable,
                            constants: c.constants.clone(),
                            stage: Stage::Responded(pendings_amount - claimable, cont_pendings),
                        }),
                    ))
                }
            } else {
                None
            }
        }
    }
}
