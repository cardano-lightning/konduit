//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{
    ExecutionUnits, Hash, Input, Output, ProtocolParameters, RedeemerPointer, Transaction, cbor,
    pallas,
};
use anyhow::anyhow;
use std::collections::{BTreeMap, BTreeSet};
use uplc::tx::SlotConfig;

// ```cddl
// vkeywitness = [ vkey, signature ]
// ```
const SIZE_OF_KEY_WITNESS: u64 = 1 // 1 byte for the 2-tuple declaration
    + (32 + 2) // 32 bytes of verification & 2 bytes of CBOR bytestring declaration
    + (64 + 2); // 64 bytes of signature + 2 bytes of CBOR bytestring declaration

/// - 1 bytes for the map key(s).
///
/// - 3 bytes for the declaration of a CBOR-Set, 1 for the tag itself, and 2 for the tag index.
///
/// - 1 to 3 bytes for the witness lists declaration. The size varies based on the number of
///   witnesses. For more than 255 witnesses, the size will be encoded over 3 bytes and allow up to
///   65535 witnesses, which should be enough...
///
/// TODO: Note that we then multiply that size by 2 to cope with both standard and byron witnesses. In
/// practice, We could potentially distinguish based on the type of witness, but that's more work.
const SIZE_OF_KEY_WITNESSES_OVERHEAD: u64 = 2 * (1 + 3 + 3);

impl Transaction {
    /// Build a transaction by repeatedly executing some building logic with different fee and execution
    /// units settings. Stops when a fixed point is reached.
    ///
    /// The final transaction has corresponding fees and execution units set.
    pub fn build<F>(
        params: &ProtocolParameters,
        resolved_inputs: &BTreeMap<Input<'_>, Output<'_>>,
        build: F,
    ) -> anyhow::Result<Self>
    where
        F: Fn(&mut Self) -> anyhow::Result<&mut Self>,
    {
        let mut attempts: usize = 0;
        let mut fee: u64 = 0;
        let mut tx: Self;
        let mut redeemers: BTreeMap<RedeemerPointer, ExecutionUnits> = BTreeMap::new();
        let mut serialized_tx: Vec<u8> = Vec::new();

        loop {
            tx = Transaction::default();

            build(tx.with_fee(fee))?;

            let required_scripts = tx.required_scripts(resolved_inputs);

            // Add a change output according to the user's chosen strategy.
            tx.with_change(resolved_inputs)?;

            // Adjust execution units calculated in previous iteration for all redeemers.
            tx.with_execution_units(&mut redeemers)?;

            // Compute & add total collateral + collateral return
            tx.with_collateral_return(resolved_inputs, params)?;

            // Add the script integrity hash, so that it counts towards the transaction size.
            tx.with_script_integrity_hash(&required_scripts, params)?;

            // Explicitly fails when there's no collaterals, but that Plutus scripts were found.
            // This informs the user of the builder that they did something wrong and forgot to set
            // one or more collateral.
            fail_on_missing_collateral(&required_scripts, tx.collaterals())?;

            // Serialise the transaction to compute its fee.
            serialized_tx.clear();
            cbor::encode(&tx, &mut serialized_tx).unwrap();

            // Re-compute execution units for all scripts, if any.
            let (total_inline_scripts_size, uplc_resolved_inputs) =
                into_uplc_inputs(&tx, resolved_inputs)?;
            redeemers = evaluate_plutus_scripts(
                &serialized_tx,
                uplc_resolved_inputs,
                &required_scripts,
                params,
            )?;

            // This estimation is a best-effort and assumes that one (non-script) input requires one signature
            // witness. This means that it possibly UNDER-estimate fees for Native-script-locked inputs;
            //
            // We don't have a solution for it at the moment.
            let estimated_fee = {
                let num_signatories = tx.required_signatories(resolved_inputs)?.len() as u64;
                let estimated_size = serialized_tx.len() as u64
                    + SIZE_OF_KEY_WITNESSES_OVERHEAD
                    + SIZE_OF_KEY_WITNESS * num_signatories;

                params.base_fee(estimated_size)
                    + params.referenced_scripts_fee(total_inline_scripts_size)
                    + total_execution_cost(params, &redeemers)
            };

            attempts += 1;

            // Check if we've reached a fixed point, or start over.
            if fee >= estimated_fee {
                break;
            } else if attempts >= 3 {
                return Err(anyhow!("transaction = {}", hex::encode(&serialized_tx))
                    .context(format!("fee = {fee}, estimated_fee = {estimated_fee}"))
                    .context(
                        "failed to build transaction: did not converge after three attempts",
                    ));
            } else {
                fee = estimated_fee;
            }
        }

        Ok(tx)
    }
}

// --------------------------------------------------------------------- Helpers

fn total_execution_cost<'a>(
    params: &'_ ProtocolParameters,
    redeemers: impl IntoIterator<Item = (&'a RedeemerPointer, &'a ExecutionUnits)>,
) -> u64 {
    redeemers.into_iter().fold(0, |acc, (_, ex_units)| {
        acc + params.price_mem(ex_units.mem()) + params.price_cpu(ex_units.cpu())
    })
}

/// Resolve specified inputs and reference inputs, and convert them into a format suitable for the
/// UPLC VM evaluation. Also returns the sum of the size of any inline scripts found in those
/// (counting multiple times the size of repeated scripts).
fn into_uplc_inputs(
    tx: &Transaction,
    resolved_inputs: &BTreeMap<Input<'_>, Output<'_>>,
) -> anyhow::Result<(u64, Vec<uplc::tx::ResolvedInput>)> {
    // Ensures that only 'known' inputs contribute to the evaluation; in case the user
    // added extra inputs to the provided UTxO which do not get correctly captured in
    // the transaction; causing the evaluation to possibly wrongly succeed.
    let known_inputs = tx
        .inputs()
        .chain(tx.reference_inputs())
        .collect::<BTreeSet<_>>();

    for input in known_inputs.iter() {
        if resolved_inputs.get(input).is_none() {
            return Err(anyhow!("unknown = {input:?}")
                .context("unknown output for specified input or reference input; found in transaction but not in provided resolved set"));
        }
    }

    let mut total_inline_scripts_size = 0;

    let uplc_resolved_inputs = resolved_inputs
        .iter()
        .filter_map(|(i, o)| {
            if known_inputs.contains(i) {
                if let Some(script) = o.script() {
                    total_inline_scripts_size += script.size();
                }

                Some(uplc::tx::ResolvedInput {
                    input: pallas::TransactionInput::from((*i).clone()),
                    output: pallas::TransactionOutput::from((*o).clone()),
                })
            } else {
                None
            }
        })
        .collect();

    Ok((total_inline_scripts_size, uplc_resolved_inputs))
}

fn fail_on_missing_collateral<'a, T>(
    redeemers: &BTreeMap<RedeemerPointer, T>,
    collaterals: impl Iterator<Item = Input<'a>>,
) -> anyhow::Result<()> {
    let mut ptrs = redeemers.keys();
    if let Some(ptr) = ptrs.next()
        && collaterals.count() == 0
    {
        let mut err = anyhow!("at {:?}", ptr);
        for ptr in ptrs {
            err = err.context(format!("at {ptr:?}"));
        }

        return Err(err.context(
            "no collaterals set, but the transaction requires at least one phase-2 script execution",
        ));
    }

    Ok(())
}

fn evaluate_plutus_scripts(
    serialized_tx: &[u8],
    resolved_inputs: Vec<uplc::tx::ResolvedInput>,
    required_scripts: &BTreeMap<RedeemerPointer, Hash<28>>,
    params: &ProtocolParameters,
) -> anyhow::Result<BTreeMap<RedeemerPointer, ExecutionUnits>> {
    if !required_scripts.is_empty() {
        // Convert to Pallas' MintedTx. Since there's no public access to the constructor of
        // 'MintedTx', we have to serialize the transaction, and deserialize it back into a
        // MintedTx directly.
        //
        // We need a MintedTx because that is the API expected from 'eval_phase_two'.
        //
        // TODO:
        //   Either:
        //    - Provide better constructors' on Pallas' side;
        //    - Adjust the 'eval_phase_two' API in the uplc crate, because there's no reason to
        //      require a MintedTx specifically.
        let minted_tx = cbor::decode(serialized_tx).unwrap();

        return Ok(uplc::tx::eval_phase_two(
            &minted_tx,
            resolved_inputs.as_slice(),
            None,
            None,
            &SlotConfig::from(params),
            false,
            |_| (),
        )
        .map_err(|e| anyhow!("required scripts = {required_scripts:?}").context(format!("{e:?}")))?
        .into_iter()
        // FIXME: The second element in the resulting pair contains the evaluation result.
        // We shall make sure that it is passing, and if it isn't, we should fail with a
        // proper error including the evaluation traces.
        .map(|(redeemer, _eval_result)| {
            (
                RedeemerPointer::from(pallas::RedeemersKey {
                    tag: redeemer.tag,
                    index: redeemer.index,
                }),
                ExecutionUnits::from(redeemer.ex_units),
            )
        })
        .collect());
    };

    Ok(BTreeMap::new())
}

// ----------------------------------------------------------------------- Tests

#[cfg(test)]
mod tests {
    use crate::{
        Address, ChangeStrategy, Input, Output, PlutusData, PlutusScript, PlutusVersion,
        ProtocolParameters, Transaction, address, address_test, cbor::ToCbor, input, mint, output,
        plutus_script, script_credential, value,
    };
    use std::{collections::BTreeMap, sync::LazyLock};

    /// Some fixture parameters, simply mimicking PreProd's parameters.
    pub static FIXTURE_PROTOCOL_PARAMETERS: LazyLock<ProtocolParameters> =
        LazyLock::new(ProtocolParameters::preprod);

    #[test]
    fn single_in_single_out() {
        let resolved_inputs = BTreeMap::from([(
            input!(
                "32b5e793d26af181cb837ab7470ba6e10e15ff638088bc6b099bb22b54b4796c",
                1
            )
            .0,
            output!(
                "addr1qxjgtdjrdj05nge3v406z46yqhp7nwc744j7sju37287sfjrcq0durn7xns7whpp6mymksagz9msf08qxqfakhc85dgq9pynjj",
                value!(
                    7933351,
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
                ),
            ),
        )]);

        let result = Transaction::build(&FIXTURE_PROTOCOL_PARAMETERS, &resolved_inputs, |tx| {
            tx
                .with_inputs(vec![
                    input!("32b5e793d26af181cb837ab7470ba6e10e15ff638088bc6b099bb22b54b4796c", 1),
                ])
                .with_outputs(vec![
                    output!(
                        "addr1q8lgqva8uleq9f3wjsnggh42d6y8vm9rvah380wq3x9djqwhy3954pmhklwxjz05vsx0qt4yw4a9275eldyrkp0c0hlqgxc7du",
                        value!(
                            6687232,
                            ("279c909f348e533da5808898f87f9a14bb2c3dfbbacccd631d927a3f", "534e454b", 1376),
                            ("a0028f350aaabe0545fdcb56b039bfb08e4bb4d8c4d7c3c7d481c235", "484f534b59", 134468443),
                        ),
                    ),
                ])
                .with_change_strategy(ChangeStrategy::as_last_output(
                    address!("addr1qxjgtdjrdj05nge3v406z46yqhp7nwc744j7sju37287sfjrcq0durn7xns7whpp6mymksagz9msf08qxqfakhc85dgq9pynjj")
                ))
                .ok()
        });

        assert!(result.is_ok(), "{result:#?}");

        // Actual minimum fee measured from constructing the serialized transaction using the
        // cardano-cli and signing it with the required keys. Somehow the cli over-estimates fee,
        // which is good, so we can assert being at least better than the cardano-cli.
        let minimum_fee = 171925;
        let cardano_cli_fee = 176369;
        let fee = result.unwrap().fee();

        assert!(
            fee >= minimum_fee && fee <= cardano_cli_fee,
            "estimated fee={fee}, minimum required={minimum_fee}, cardano-cli's estimation={cardano_cli_fee}",
        );
    }

    #[test]
    fn mint_tokens() {
        let resolved_inputs = BTreeMap::from([(
            input!(
                "d62db0b98b6df96645eec19d4728b385592fc531736abd987eb6490510c5ba50",
                0
            )
            .0,
            output!(
                "addr1qxu84ftxpzh3zd8p9awp2ytwzk5exj0fxcj7paur4kd4ytun36yuhgl049rxhhuckm2lpq3rmz5dcraddyl45d6xgvqqsp504c",
                value!(102049379)
            ),
        )]);

        let result = Transaction::build(&FIXTURE_PROTOCOL_PARAMETERS, &resolved_inputs, |tx| {
            tx
                .with_inputs(vec![
                    input!("d62db0b98b6df96645eec19d4728b385592fc531736abd987eb6490510c5ba50", 0),
                ])
                .with_collaterals(vec![
                    input!("d62db0b98b6df96645eec19d4728b385592fc531736abd987eb6490510c5ba50", 0).0,
                ])
                .with_change_strategy(ChangeStrategy::as_last_output(
                    address!(
                        "addr1qxu84ftxpzh3zd8p9awp2ytwzk5exj0fxcj7paur4kd4ytun36yuhgl049rxhhuckm2lpq3rmz5dcraddyl45d6xgvqqsp504c",
                    )
                ))
                .with_mint(mint!(
                    (
                        "5fb286e39c3cda5a5abd17501c17b01987ebfa282df129c4df1bf27e",
                        "e29ca82073756d6d6974203230323520646973636f756e74207368617264",
                        100_i64,
                        PlutusData::list(vec![]),
                    ),
                ))
                .with_plutus_scripts(vec![
                    plutus_script!(
                        PlutusVersion::V3,
                        "59015d01010029800aba2aba1aba0aab9faab9eaab9dab9a48888889\
                         6600264653001300800198041804800cc0200092225980099b874800\
                         0c01cdd500144c8c966002003168acc004c0380062b3001337106eb4\
                         c028c03400520008a51899198008009bac300e300b375400844b3001\
                         0018a508acc004cdd7980798061baa300f0014c127d8799f5820d62d\
                         b0b98b6df96645eec19d4728b385592fc531736abd987eb6490510c5\
                         ba5000ff008a518998010011808000a014403480422c805900b192cc\
                         004cdc3a400460126ea8006297adef6c6089bab300d300a375400280\
                         40c8cc004004dd59806980718071807180718051baa0032259800800\
                         c5300103d87a8000899192cc004cdc8802800c56600266e3c0140062\
                         66e9520003300f300d0024bd7045300103d87a8000402d1330040043\
                         011003402c6eb8c02c004c03800500c1bae300b30083754005164018\
                         300800130033754011149a26cac80081",
                    )
                ])
                .ok()
        });

        assert!(result.is_ok(), "{result:#?}");

        // Actual minimum fee measured from constructing the serialized transaction using the
        // cardano-cli and signing it with the required keys. Somehow the cli over-estimates fee,
        // which is good, so we can assert being at least better than the cardano-cli.
        //
        // See the transaction: https://cardanoscan.io/transaction/ff3c022d38cfc18e66c45d14823c7b948de77ed3ca10d07cabecc57c1f44b707
        let minimum_fee = 194365;
        let cardano_cli_fee = 205850;
        let fee = result.unwrap().fee();

        assert!(
            fee >= minimum_fee && fee <= cardano_cli_fee,
            "estimated fee={fee}, minimum required={minimum_fee}, cardano-cli's estimation={cardano_cli_fee}",
        );
    }

    static ALWAYS_SUCCEED_ADDRESS: LazyLock<Address<'static, address::Any>> = LazyLock::new(|| {
        Address::from(address_test!(script_credential!(
            "bd3ae991b5aafccafe5ca70758bd36a9b2f872f57f6d3a1ffa0eb777"
        )))
    });

    static ALWAYS_SUCCEED_SCRIPT: LazyLock<PlutusScript> =
        LazyLock::new(|| plutus_script!(PlutusVersion::V3, "5101010023259800a518a4d136564004ae69"));

    #[test]
    fn full_lifecycle() {
        let my_address: Address<'_, address::Any> = address!(
            "addr_test1qzpvzu5atl2yzf9x4eetekuxkm5z02kx5apsreqq8syjum6274ase8lkeffp39narear74ed0nf804e5drfm9l99v4eq3ecz8t"
        );

        let mut resolved_inputs = BTreeMap::from([(
            input!(
                "c984c8bf52a141254c714c905b2d27b432d4b546f815fbc2fea7b9da6e490324",
                1
            )
            .0,
            Output::new(my_address.clone(), value!(47321123)),
        )]);

        let deploy_script =
            Transaction::build(&FIXTURE_PROTOCOL_PARAMETERS, &resolved_inputs, |tx| {
                tx.with_inputs(vec![input!(
                    "c984c8bf52a141254c714c905b2d27b432d4b546f815fbc2fea7b9da6e490324",
                    1
                )])
                .with_outputs(vec![
                    Output::to(my_address.clone())
                        .with_plutus_script(ALWAYS_SUCCEED_SCRIPT.clone()),
                ])
                .with_change_strategy(ChangeStrategy::as_last_output(my_address.clone()))
                .ok()
            })
            .unwrap();

        assert_eq!(
            hex::encode(deploy_script.to_cbor()),
            "84a300d9010281825820c984c8bf52a141254c714c905b2d27b432d4b546f815fbc\
             2fea7b9da6e490324010182a30058390082c1729d5fd44124a6ae72bcdb86b6e827\
             aac6a74301e4003c092e6f4af57b0c9ff6ca5218967d1e7a3f572d7cd277d73468d\
             3b2fca56572011a001092a803d818558203525101010023259800a518a4d1365640\
             04ae69a20058390082c1729d5fd44124a6ae72bcdb86b6e827aac6a74301e4003c0\
             92e6f4af57b0c9ff6ca5218967d1e7a3f572d7cd277d73468d3b2fca56572011a02\
             bee626021a00029755a0f5f6",
            "deploy_script no longer matches expected bytes."
        );

        let deploy_outputs = deploy_script.outputs().collect::<Vec<_>>();
        resolved_inputs = BTreeMap::from([
            (Input::new(deploy_script.id(), 0), deploy_outputs[0].clone()),
            (Input::new(deploy_script.id(), 1), deploy_outputs[1].clone()),
        ]);

        let pay_to_script =
            Transaction::build(&FIXTURE_PROTOCOL_PARAMETERS, &resolved_inputs, |tx| {
                tx.with_inputs(vec![(Input::new(deploy_script.id(), 1), None)])
                    .with_outputs(vec![Output::new(
                        ALWAYS_SUCCEED_ADDRESS.clone(),
                        value!(10_000_000),
                    )])
                    .with_change_strategy(ChangeStrategy::as_last_output(my_address.clone()))
                    .ok()
            })
            .unwrap();

        assert_eq!(
            hex::encode(pay_to_script.to_cbor()),
            "84a300d901028182582026dad69d058e6aed8dd112266c8cda84ecca7c8132b535c\
             65697f2409d0d2e80010182a200581d70bd3ae991b5aafccafe5ca70758bd36a9b2\
             f872f57f6d3a1ffa0eb777011a00989680a20058390082c1729d5fd44124a6ae72b\
             cdb86b6e827aac6a74301e4003c092e6f4af57b0c9ff6ca5218967d1e7a3f572d7c\
             d277d73468d3b2fca56572011a0223c16d021a00028e39a0f5f6",
            "pay_to_script no longer matches expected bytes."
        );

        let pay_to_script_outputs = pay_to_script.outputs().collect::<Vec<_>>();
        resolved_inputs = BTreeMap::from([
            (Input::new(deploy_script.id(), 0), deploy_outputs[0].clone()),
            (
                Input::new(pay_to_script.id(), 0),
                pay_to_script_outputs[0].clone(),
            ),
            (
                Input::new(pay_to_script.id(), 1),
                pay_to_script_outputs[1].clone(),
            ),
        ]);

        let spend_from_script =
            Transaction::build(&FIXTURE_PROTOCOL_PARAMETERS, &resolved_inputs, |tx| {
                tx.with_inputs(vec![(
                    Input::new(pay_to_script.id(), 0),
                    Some(PlutusData::list(vec![])),
                )])
                .with_reference_inputs(vec![(Input::new(deploy_script.id(), 0))])
                .with_collaterals(vec![Input::new(pay_to_script.id(), 1)])
                .with_change_strategy(ChangeStrategy::as_last_output(
                    ALWAYS_SUCCEED_ADDRESS.clone(),
                ))
                .ok()
            })
            .unwrap();

        assert_eq!(
            hex::encode(spend_from_script.to_cbor()),
            "84a800d901028182582004aca0496e336f36f219a1ddb8298555a1b166a988990b8\
             427ec3ff292fc6b7a000181a200581d70bd3ae991b5aafccafe5ca70758bd36a9b2\
             f872f57f6d3a1ffa0eb777011a0095ef65021a0002a71b0b5820d545623b07e425a\
             55262585d2b5e8aaee16230fd1434e790fa4511da4bf8a4550dd901028182582004\
             aca0496e336f36f219a1ddb8298555a1b166a988990b8427ec3ff292fc6b7a0110a\
             20058390082c1729d5fd44124a6ae72bcdb86b6e827aac6a74301e4003c092e6f4a\
             f57b0c9ff6ca5218967d1e7a3f572d7cd277d73468d3b2fca56572011a0223c16d1\
             10012d901028182582026dad69d058e6aed8dd112266c8cda84ecca7c8132b535c6\
             5697f2409d0d2e8000a105a18200008280821906411a0004d2f5f5f6",
            "spend_from_script no longer matches expected bytes."
        );
    }
}
