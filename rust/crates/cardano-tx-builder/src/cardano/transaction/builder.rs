//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{
    Address, ExecutionUnits, Hash, Input, Output, ProtocolParameters, RedeemerPointer, Transaction,
    Value, cbor, pallas,
};
use anyhow::anyhow;
use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Deref,
};
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
            fail_on_missing_collateral(&required_scripts, &tx.collaterals())?;

            // Serialise the transaction to compute its fee.
            serialized_tx.clear();
            cbor::encode(&tx, &mut serialized_tx).unwrap();

            // Re-compute execution units for all scripts, if any.
            redeemers = evaluate_plutus_scripts(
                &serialized_tx,
                into_uplc_inputs(&tx, resolved_inputs),
                &required_scripts,
                params,
            )?;

            // This estimation is a best-effort and assumes that one input requires one signature
            // witness. This means that:
            //
            // - it slightly OVER-estimate fees for Plutus-script-locked inputs;
            // - it slightly OVER-estimate fees for duplicate signers;
            // - it possibly UNDER-estimate fees for Native-script-locked inputs;
            let estimated_fee = {
                let num_signatories = (tx.inputs().len() + tx.specified_signatories().len()) as u64;
                let estimated_size = serialized_tx.len() as u64
                    + SIZE_OF_KEY_WITNESSES_OVERHEAD
                    + SIZE_OF_KEY_WITNESS * num_signatories;
                params.base_fee(estimated_size) + total_execution_cost(params, &redeemers)
            };

            attempts += 1;

            // Check if we've reached a fixed point, or start over.
            if fee >= estimated_fee {
                break;
            } else if attempts >= 3 {
                return Err(anyhow!("transaction = {}", hex::encode(&serialized_tx))
                    .context(format!("fee = {fee}, estimated_fee = {estimated_fee}"))
                    .context(
                        "failed to build transaction: did not converge after three attempts.",
                    ));
            } else {
                fee = estimated_fee;
            }
        }

        Ok(tx)
    }

    fn with_script_integrity_hash(
        &mut self,
        required_scripts: &BTreeMap<RedeemerPointer, Hash<28>>,
        params: &ProtocolParameters,
    ) -> anyhow::Result<()> {
        if let Some(hash) = self.script_integrity_hash(params) {
            self.inner.transaction_body.script_data_hash = Some(pallas::Hash::from(hash));
        } else if !required_scripts.is_empty() {
            let mut scripts = required_scripts.iter();

            let (ptr, hash) = scripts.next().unwrap(); // Safe because it's not empty
            let mut err = anyhow!("required_scripts = {ptr:?} -> {hash}");
            for (ptr, hash) in scripts {
                err = err.context(format!("required_scripts = {ptr:?} -> {hash}"));
            }

            return Err(err.context("couldn't compute required script integrity hash: datums and redeemers are missing from the transaction."));
        }

        Ok(())
    }

    fn with_execution_units(
        &mut self,
        redeemers: &mut BTreeMap<RedeemerPointer, ExecutionUnits>,
    ) -> anyhow::Result<()> {
        if let Some(declared_redeemers) =
            std::mem::take(&mut self.inner.transaction_witness_set.redeemer)
        {
            match declared_redeemers {
                pallas::Redeemers::List(..) => {
                    unreachable!("found redeemers encoded as list: impossible with this library.")
                }

                pallas::Redeemers::Map(kv) => {
                    self.inner.transaction_witness_set.redeemer =
                        pallas::NonEmptyKeyValuePairs::from_vec(
                            kv.into_iter()
                                .map(|(key, mut value)| {
                                    let ptr = RedeemerPointer::from(key.clone());
                                    // If we have already computed the correct execution units
                                    // for that redeemer in a previous round, we can adjust
                                    // them.
                                    if let Some(ex_units) = redeemers.remove(&ptr) {
                                        value.ex_units = pallas::ExUnits::from(ex_units);
                                    }
                                    (key, value)
                                })
                                .collect(),
                        )
                        .map(pallas::Redeemers::from)
                }
            }
        }

        // We should technically have consumed all redeemers.
        if !redeemers.is_empty() {
            return Err(
                anyhow!("extraneous redeemers in transaction; not required by any script")
                    .context(format!("extra={:?}", redeemers.keys().collect::<Vec<_>>())),
            );
        }

        Ok(())
    }

    fn with_change(
        &mut self,
        resolved_inputs: &BTreeMap<Input<'_>, Output<'_>>,
    ) -> anyhow::Result<()> {
        let mut change = Value::default();

        // Add inputs to the change balance
        self.inputs()
            .iter()
            .try_fold(&mut change, |total_input, input| {
                let output = resolved_inputs.get(input).ok_or_else(|| {
                    anyhow!("unknown input, not present in resolved set")
                        .context(format!("input={input:?}"))
                })?;

                Ok::<_, anyhow::Error>(total_input.add(output.value().deref().as_ref()))
            })?;

        // Partition mint quantities between mint & burn
        let (mint, burn) = self.mint().assets().clone().into_iter().fold(
            (BTreeMap::new(), BTreeMap::new()),
            |(mut mint, mut burn), (policy, assets)| {
                let mut minted_assets = BTreeMap::new();
                let mut burned_assets = BTreeMap::new();

                for (asset_name, quantity) in assets {
                    if quantity > 0 {
                        minted_assets.insert(asset_name, quantity as u64);
                    } else {
                        burned_assets.insert(asset_name, (-quantity) as u64);
                    }
                }

                if !minted_assets.is_empty() {
                    mint.insert(policy, minted_assets);
                }

                if !burned_assets.is_empty() {
                    burn.insert(policy, burned_assets);
                }

                (mint, burn)
            },
        );

        // Add minted tokens to the change balance
        change.add(&Value::default().with_assets(mint));

        // Subtract burned tokens from the change balance
        change
            .checked_sub(&Value::default().with_assets(burn))
            .map_err(|e| e.context("insufficient balance; spending more than available"))?;

        // Subtract all outputs from the change balance
        self.outputs()
            .iter()
            .try_fold(&mut change, |total_output, output| {
                total_output.checked_sub(output.value().deref().as_ref())
            })
            .map_err(|e| e.context("insufficient balance; spending more than available"))?;

        // Subtract the transaction fee as well
        change
            .checked_sub(&Value::new(self.fee()))
            .map_err(|e| e.context("insufficient balance; spending more than available"))?;

        let body = &self.inner.transaction_body;

        assert!(
            body.certificates.is_none(),
            "found certificates in transaction: not supported yet",
        );

        assert!(
            body.withdrawals.is_none(),
            "found withdrawals in transaction: not supported yet",
        );

        assert!(
            body.treasury_value.is_none(),
            "found treasury donation in transaction: not supported yet",
        );

        assert!(
            body.proposal_procedures.is_none(),
            "found proposals in transaction: not supported yet",
        );

        if !change.is_empty() {
            self.with_change_output(change)?;
        }

        Ok(())
    }

    fn with_collateral_return(
        &mut self,
        resolved_inputs: &BTreeMap<Input<'_>, Output<'_>>,
        params: &ProtocolParameters,
    ) -> anyhow::Result<()> {
        let (mut total_collateral_value, opt_return_address): (
            Value<u64>,
            Option<Address<'static, _>>,
        ) = self
            .collaterals()
            .iter()
            .map(|input| {
                resolved_inputs.get(input).ok_or_else(|| {
                    anyhow!("unknown collateral input").context(format!("reference={input:?}"))
                })
            })
            .try_fold(
                (Value::new(0), None),
                |(mut total, address), maybe_output| {
                    let output = maybe_output?;
                    total.add(output.value().deref().as_ref());
                    // It is arbitrary, but we use the source address of the first collateral as the
                    // destination of the collateral change. Collaterals can't be script, so this is
                    // relatively safe as the ledger enforces that the key is known at the time the
                    // transaction is constructed.
                    Ok::<_, anyhow::Error>((
                        total,
                        address.or_else(|| Some(output.address().to_owned())),
                    ))
                },
            )?;

        if let Some(return_address) = opt_return_address {
            let minimum_required_collateral =
                (self.fee() as f64 * params.collateral_coefficient()).ceil() as u64;

            total_collateral_value
                .checked_sub(&Value::new(minimum_required_collateral))
                .map_err(|e| e.context("insufficient collateral inputs"))?;

            self.inner.transaction_body.total_collateral = Some(minimum_required_collateral);
            self.inner.transaction_body.collateral_return = Some(pallas::TransactionOutput::from(
                // A bit misleading but 'total_collateral_value' now refers to the total amount brought
                // in, minus the the minimum required by the protocol, left out.
                Output::new(return_address, total_collateral_value),
            ));
        }

        Ok(())
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

fn into_uplc_inputs(
    tx: &Transaction,
    resolved_inputs: &BTreeMap<Input<'_>, Output<'_>>,
) -> Vec<uplc::tx::ResolvedInput> {
    // Ensures that only 'known' inputs contribute to the evaluation; in case the user
    // added extra inputs to the provided UTxO which do not get correctly captured in
    // the transaction; causing the evaluation to possibly wrongly succeed.
    let known_inputs = tx
        .inputs()
        .into_iter()
        .chain(tx.reference_inputs())
        .collect::<BTreeSet<_>>();

    resolved_inputs
        .iter()
        .filter_map(|(i, o)| {
            if known_inputs.contains(i) {
                Some(uplc::tx::ResolvedInput {
                    input: pallas::TransactionInput::from((*i).clone()),
                    output: pallas::TransactionOutput::from((*o).clone()),
                })
            } else {
                None
            }
        })
        .collect()
}

fn fail_on_missing_collateral<'a, T>(
    redeemers: &BTreeMap<RedeemerPointer, T>,
    collaterals: &[Input<'a>],
) -> anyhow::Result<()> {
    let mut ptrs = redeemers.keys();
    if let Some(ptr) = ptrs.next()
        && collaterals.is_empty()
    {
        let mut err = anyhow!("at {:?}", ptr);
        for ptr in ptrs {
            err = err.context(format!("at {ptr:?}"));
        }

        return Err(err.context(
            "no collaterals set, but the transaction requires at least one phase-2 script execution.",
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

    /// Some fixture parameters, simply mimicking Mainnet/PreProd's parameters.
    pub static FIXTURE_PROTOCOL_PARAMETERS: LazyLock<ProtocolParameters> = LazyLock::new(|| {
        ProtocolParameters::default()
            .with_fee_per_byte(44)
            .with_fee_constant(155381)
            .with_execution_price_mem(0.0577)
            .with_execution_price_cpu(7.21e-05)
            .with_start_time(1654041600)
            .with_first_shelley_slot(86400)
            .with_plutus_v3_cost_model([
                100788, 420, 1, 1, 1000, 173, 0, 1, 1000, 59957, 4, 1, 11183, 32, 201305, 8356, 4,
                16000, 100, 16000, 100, 16000, 100, 16000, 100, 16000, 100, 16000, 100, 100, 100,
                16000, 100, 94375, 32, 132994, 32, 61462, 4, 72010, 178, 0, 1, 22151, 32, 91189,
                769, 4, 2, 85848, 123203, 7305, -900, 1716, 549, 57, 85848, 0, 1, 1, 1000, 42921,
                4, 2, 24548, 29498, 38, 1, 898148, 27279, 1, 51775, 558, 1, 39184, 1000, 60594, 1,
                141895, 32, 83150, 32, 15299, 32, 76049, 1, 13169, 4, 22100, 10, 28999, 74, 1,
                28999, 74, 1, 43285, 552, 1, 44749, 541, 1, 33852, 32, 68246, 32, 72362, 32, 7243,
                32, 7391, 32, 11546, 32, 85848, 123203, 7305, -900, 1716, 549, 57, 85848, 0, 1,
                90434, 519, 0, 1, 74433, 32, 85848, 123203, 7305, -900, 1716, 549, 57, 85848, 0, 1,
                1, 85848, 123203, 7305, -900, 1716, 549, 57, 85848, 0, 1, 955506, 213312, 0, 2,
                270652, 22588, 4, 1457325, 64566, 4, 20467, 1, 4, 0, 141992, 32, 100788, 420, 1, 1,
                81663, 32, 59498, 32, 20142, 32, 24588, 32, 20744, 32, 25933, 32, 24623, 32,
                43053543, 10, 53384111, 14333, 10, 43574283, 26308, 10, 16000, 100, 16000, 100,
                962335, 18, 2780678, 6, 442008, 1, 52538055, 3756, 18, 267929, 18, 76433006, 8868,
                18, 52948122, 18, 1995836, 36, 3227919, 12, 901022, 1, 166917843, 4307, 36, 284546,
                36, 158221314, 26549, 36, 74698472, 36, 333849714, 1, 254006273, 72, 2174038, 72,
                2261318, 64571, 4, 207616, 8310, 4, 1293828, 28716, 63, 0, 1, 1006041, 43623, 251,
                0, 1, 100181, 726, 719, 0, 1, 100181, 726, 719, 0, 1, 100181, 726, 719, 0, 1,
                107878, 680, 0, 1, 95336, 1, 281145, 18848, 0, 1, 180194, 159, 1, 1, 158519, 8942,
                0, 1, 159378, 8813, 0, 1, 107490, 3298, 1, 106057, 655, 1, 1964219, 24520, 3,
            ])
    });

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

        resolved_inputs = BTreeMap::from([
            (
                Input::new(deploy_script.id(), 0),
                deploy_script.outputs()[0].clone(),
            ),
            (
                Input::new(deploy_script.id(), 1),
                deploy_script.outputs()[1].clone(),
            ),
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

        resolved_inputs = BTreeMap::from([
            (
                Input::new(deploy_script.id(), 0),
                deploy_script.outputs()[0].clone(),
            ),
            (
                Input::new(pay_to_script.id(), 0),
                pay_to_script.outputs()[0].clone(),
            ),
            (
                Input::new(pay_to_script.id(), 1),
                pay_to_script.outputs()[1].clone(),
            ),
        ]);

        let spend_from_script =
            Transaction::build(&FIXTURE_PROTOCOL_PARAMETERS, &resolved_inputs, |tx| {
                tx.with_inputs(vec![(
                    Input::new(pay_to_script.id(), 0),
                    Some(PlutusData::list(vec![])),
                )])
                .with_collaterals(vec![Input::new(pay_to_script.id(), 1)])
                .with_change_strategy(ChangeStrategy::as_last_output(
                    ALWAYS_SUCCEED_ADDRESS.clone(),
                ))
                .with_plutus_scripts(vec![ALWAYS_SUCCEED_SCRIPT.clone()])
                .ok()
            })
            .unwrap();

        assert_eq!(
            hex::encode(spend_from_script.to_cbor()),
            "84a700d901028182582004aca0496e336f36f219a1ddb8298555a1b166a988990b8\
             427ec3ff292fc6b7a000181a200581d70bd3ae991b5aafccafe5ca70758bd36a9b2\
             f872f57f6d3a1ffa0eb777011a0095f2af021a0002a3d10b5820d545623b07e425a\
             55262585d2b5e8aaee16230fd1434e790fa4511da4bf8a4550dd901028182582004\
             aca0496e336f36f219a1ddb8298555a1b166a988990b8427ec3ff292fc6b7a0110a\
             20058390082c1729d5fd44124a6ae72bcdb86b6e827aac6a74301e4003c092e6f4a\
             f57b0c9ff6ca5218967d1e7a3f572d7cd277d73468d3b2fca56572011a021fcbb31\
             11a0003f5baa205a18200008280821906411a0004d2f507d9010281525101010023\
             259800a518a4d136564004ae69f5f6",
            "spend_from_script no longer matches expected bytes."
        );
    }
}
