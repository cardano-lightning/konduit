//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{
    ExecutionUnits, Hash, Input, Output, ProtocolParameters, RedeemerPointer, Transaction,
    cardano::transaction::{IsTransactionBodyState, state},
    cbor, pallas,
};
use anyhow::anyhow;
use std::{
    collections::{BTreeMap, BTreeSet},
    marker::PhantomData,
    mem,
};
use uplc::tx::SlotConfig;

// ```cddl
// vkeywitness = [ vkey, signature ]
// ```
const SIZE_OF_KEY_WITNESS: u64 = 1 // 1 byte for the 2-tuple declaration
    + (32 + 2) // 32 bytes of verification & 2 bytes of CBOR bytestring declaration
    + (64 + 2); // 64 bytes of signature + 2 bytes of CBOR bytestring declaration

// - 1 bytes for the map key(s).
//
// - 3 bytes for the declaration of a CBOR-Set, 1 for the tag itself, and 2 for the tag index.
//
// - 1 to 3 bytes for the witness lists declaration. The size varies based on the number of
//   witnesses. For more than 255 witnesses, the size will be encoded over 3 bytes and allow up to
//   65535 witnesses, which should be enough...
//
// TODO: Note that we then multiply that size by 2 to cope with both standard and byron witnesses. In
// practice, We could potentially distinguish based on the type of witness, but that's more work.
const SIZE_OF_KEY_WITNESSES_OVERHEAD: u64 = 2 * (1 + 3 + 3);

impl Transaction<state::InConstruction> {
    /// Build a transaction by repeatedly executing some building logic with different fee and execution
    /// units settings. Stops when a fixed point is reached.
    ///
    /// The final transaction has corresponding fees, execution units, collateral return and script
    /// integrity hash set.
    ///
    /// # notes
    ///
    /// - any input specified in the transaction (either as input, reference input or collateral) must be present in the provided `resolved_inputs`.
    /// - `with_change_strategy` must **always** be present for there's no default change strategy.
    /// - while it is allowed (for whatever reasons), you aren't expected to fiddle with fees here;
    ///   this is entirely managed by the `build` loop.
    ///
    /// # examples
    ///
    /// ## A simple transaction from a single input to a single recipient, with change.
    ///
    /// ```rust
    /// # use cardano_tx_builder::{ChangeStrategy, ProtocolParameters, Transaction, input, output, value, address};
    /// # use std::collections::btree_map::BTreeMap;
    /// // The available UTxO, typically fetched from a blockchain provider or an indexer.
    /// let resolved_inputs = BTreeMap::from([
    ///   (
    ///     input!("32b5e793d26af181cb837ab7470ba6e10e15ff638088bc6b099bb22b54b4796c", 1).0,
    ///     output!(
    ///       "addr1vx7n46v3kk40ejh7tjnswk9ax65m97rj74lk6wsllg8twacak3e47",
    ///       value!(10_000_000),
    ///     ),
    ///   )
    /// ]);
    ///
    /// // Send a minimum lovelace value to an arbitrary address, and the change back to the
    /// // sender.
    /// assert_eq!(
    ///   Transaction::build(&ProtocolParameters::mainnet(), &resolved_inputs, |tx| tx
    ///     .with_inputs(vec![
    ///       input!("32b5e793d26af181cb837ab7470ba6e10e15ff638088bc6b099bb22b54b4796c", 1),
    ///     ])
    ///     .with_outputs(vec![
    ///       output!("addr1wyhcwt6h7mf6rlaqadmhh5awnyd44t7v4lju5ur430fk4xczzq8aw"),
    ///     ])
    ///     .with_change_strategy(ChangeStrategy::as_last_output(
    ///       address!("addr1vx7n46v3kk40ejh7tjnswk9ax65m97rj74lk6wsllg8twacak3e47"),
    ///     ))
    ///     .ok()
    ///   ).unwrap().to_string(),
    ///   "Transaction (id = 6f8e53f61fe0a709e1c895c1bd9487e555779de0902ad25377d4de1df48f08b8) { \
    ///      inputs: [Input(32b5e793d26af181cb837ab7470ba6e10e15ff638088bc6b099bb22b54b4796c#1)], \
    ///      outputs: [\
    ///        Output { \
    ///          address: addr1wyhcwt6h7mf6rlaqadmhh5awnyd44t7v4lju5ur430fk4xczzq8aw, \
    ///          value: Value { lovelace: 857690 } \
    ///        }, \
    ///        Output { \
    ///          address: addr1vx7n46v3kk40ejh7tjnswk9ax65m97rj74lk6wsllg8twacak3e47, \
    ///          value: Value { lovelace: 8976061 } \
    ///        }\
    ///      ], \
    ///      fee: 166249 \
    ///    }"
    /// );
    /// ```
    ///
    /// ## Minting assets using a plutus script.
    ///
    ///
    /// ```rust
    /// # use cardano_tx_builder::{ChangeStrategy, PlutusData, PlutusVersion, ProtocolParameters, Transaction, address, assets, input, output, plutus_script, value};
    /// # use std::collections::btree_map::BTreeMap;
    /// let resolved_inputs = BTreeMap::from([
    ///   (
    ///     input!("32b5e793d26af181cb837ab7470ba6e10e15ff638088bc6b099bb22b54b4796c", 1).0,
    ///     output!(
    ///       "addr1vx7n46v3kk40ejh7tjnswk9ax65m97rj74lk6wsllg8twacak3e47",
    ///       value!(10_000_000),
    ///     ),
    ///   )
    /// ]);
    ///
    /// // Notice the absence of outputs and of collateral return. We let the builder handler those for us.
    /// assert_eq!(
    ///   Transaction::build(&ProtocolParameters::mainnet(), &resolved_inputs, |tx| tx
    ///     .with_inputs(vec![
    ///       input!("32b5e793d26af181cb837ab7470ba6e10e15ff638088bc6b099bb22b54b4796c", 1),
    ///     ])
    ///     .with_collaterals(vec![
    ///       input!("32b5e793d26af181cb837ab7470ba6e10e15ff638088bc6b099bb22b54b4796c", 1).0,
    ///     ])
    ///     .with_change_strategy(ChangeStrategy::as_last_output(
    ///       address!("addr1vx7n46v3kk40ejh7tjnswk9ax65m97rj74lk6wsllg8twacak3e47"),
    ///     ))
    ///     .with_mint(assets!(
    ///       (
    ///         "bd3ae991b5aafccafe5ca70758bd36a9b2f872f57f6d3a1ffa0eb777",
    ///         "7768617465766572",
    ///         100_i64,
    ///         PlutusData::list(vec![]),
    ///       ),
    ///     ))
    ///     .with_plutus_scripts(vec![
    ///       plutus_script!(PlutusVersion::V3, "5101010023259800a518a4d136564004ae69")
    ///     ])
    ///     .ok()
    ///   ).unwrap().to_string(),
    ///   "Transaction (id = 4b61e0fb18523e5c28f124b98c87de7b6d6cb7d049d2c75c94327e357bbbd54d) { \
    ///      inputs: [Input(32b5e793d26af181cb837ab7470ba6e10e15ff638088bc6b099bb22b54b4796c#1)], \
    ///      outputs: [\
    ///        Output { \
    ///          address: addr1vx7n46v3kk40ejh7tjnswk9ax65m97rj74lk6wsllg8twacak3e47, \
    ///          value: Value { \
    ///            lovelace: 9824351, \
    ///            assets: {bd3ae991b5aafccafe5ca70758bd36a9b2f872f57f6d3a1ffa0eb777: {whatever: 100}} \
    ///          } \
    ///        }\
    ///      ], \
    ///      fee: 175649, \
    ///      mint: Value { \
    ///        lovelace: 0, \
    ///        assets: {bd3ae991b5aafccafe5ca70758bd36a9b2f872f57f6d3a1ffa0eb777: {whatever: 100}} \
    ///      }, \
    ///      script_integrity_hash: b5a66ea46c7628f9b151d6e029f322058d9dff3793a98f3cfed6e21ed7064c4f, \
    ///      collaterals: [Input(32b5e793d26af181cb837ab7470ba6e10e15ff638088bc6b099bb22b54b4796c#1)], \
    ///      collateral_return: Output { \
    ///        address: addr1vx7n46v3kk40ejh7tjnswk9ax65m97rj74lk6wsllg8twacak3e47, \
    ///        value: Value { lovelace: 9736526 } \
    ///       }, \
    ///       total_collateral: 263474, \
    ///       scripts: [v3(bd3ae991b5aafccafe5ca70758bd36a9b2f872f57f6d3a1ffa0eb777)], \
    ///       redeemers: {\
    ///         Mint(0): Redeemer(CBOR(80), ExecutionUnits { mem: 1601, cpu: 316149 })\
    ///       } \
    ///    }",
    /// );
    pub fn build<F>(
        params: &ProtocolParameters,
        resolved_inputs: &BTreeMap<Input, Output>,
        build: F,
    ) -> anyhow::Result<Transaction<state::ReadyForSigning>>
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

            // Compute & add total collateral + collateral return
            tx.with_collateral_return(resolved_inputs, params)?;

            // Adjust execution units calculated in previous iteration for all redeemers.
            tx.with_execution_units(&mut redeemers)?;

            // Add the script integrity hash, so that it counts towards the transaction size.
            tx.with_script_integrity_hash(&required_scripts, params)?;

            // Explicitly fails when there's no collaterals, but that Plutus scripts were found.
            // This informs the user of the builder that they did something wrong and forgot to set
            // one or more collateral.
            fail_on_missing_collateral(&required_scripts, tx.collaterals())?;

            // Fails when any output is below its minimum Ada value. We specifically call this
            // after 'with_change' so that it's possible for users to create outputs that are
            // initially too small but get repleted when distributing change.
            //
            // The builder shall never produce an invalid change output that is too small (i.e. the
            // 'change' given to the change strategy callback is always sufficient to cover for a
            // full new output). But users may pre-define such outputs; hence the safeguard here.
            fail_on_insufficiently_funded_outputs(tx.outputs())?;

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

        Ok(Transaction {
            inner: tx.inner,
            change_strategy: tx.change_strategy,
            state: PhantomData,
        })
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
fn into_uplc_inputs<State: IsTransactionBodyState>(
    tx: &Transaction<State>,
    resolved_inputs: &BTreeMap<Input, Output>,
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
            return Err(anyhow!("unknown = {input}")
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

fn fail_on_missing_collateral<T>(
    redeemers: &BTreeMap<RedeemerPointer, T>,
    collaterals: impl Iterator<Item = Input>,
) -> anyhow::Result<()> {
    let mut ptrs = redeemers.keys();
    if let Some(ptr) = ptrs.next()
        && collaterals.count() == 0
    {
        let mut err = anyhow!("at {}", ptr);
        for ptr in ptrs {
            err = err.context(format!("at {ptr}"));
        }

        return Err(err.context(
            "no collaterals set, but the transaction requires at least one phase-2 script execution",
        ));
    }

    Ok(())
}

fn fail_on_insufficiently_funded_outputs(
    outputs: impl Iterator<Item = Output>,
) -> anyhow::Result<()> {
    let mut err_opt: Option<anyhow::Error> = None;

    for (ix, output) in outputs.enumerate() {
        let allocated = output.value().lovelace();
        let minimum_required = output.min_acceptable_value();
        if allocated < minimum_required {
            if let Some(err) = mem::take(&mut err_opt) {
                err_opt = Some(err.context(format!("at output index {ix}: allocated={allocated}, minimum required={minimum_required}")));
            } else {
                err_opt = Some(anyhow!(
                    "at output index {ix}: allocated={allocated}, minimum required={minimum_required}"
                ));
            }
        }
    }

    if let Some(err) = err_opt {
        return Err(
            err.context("insufficiently provisioned output(s): not enough lovelace allocated")
        );
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
        ProtocolParameters, Transaction, address, address::kind::*, address_test, assets,
        cbor::ToCbor, hash, input, output, plutus_data, plutus_script, script_credential, value,
    };
    use std::{cell::LazyCell, collections::BTreeMap, sync::LazyLock};

    /// Some fixture parameters, simply mimicking PreProd's parameters.
    pub static FIXTURE_PROTOCOL_PARAMETERS: LazyLock<ProtocolParameters> =
        LazyLock::new(ProtocolParameters::preprod);

    #[allow(clippy::declare_interior_mutable_const)]
    const ALWAYS_SUCCEED_ADDRESS: LazyCell<Address<Any>> = LazyCell::new(|| {
        Address::from(address_test!(script_credential!(
            "bd3ae991b5aafccafe5ca70758bd36a9b2f872f57f6d3a1ffa0eb777"
        )))
    });

    #[allow(clippy::declare_interior_mutable_const)]
    const ALWAYS_SUCCEED_SCRIPT: LazyCell<PlutusScript> =
        LazyCell::new(|| plutus_script!(PlutusVersion::V3, "5101010023259800a518a4d136564004ae69"));

    #[test]
    fn spend_from_datum_hash() {
        let always_succeed_script = ALWAYS_SUCCEED_SCRIPT;
        let always_succeed_address = ALWAYS_SUCCEED_ADDRESS;

        let resolved_inputs = [
            (
                input!(
                    "d62db0b98b6df96645eec19d4728b385592fc531736abd987eb6490510c5ba50",
                    0,
                ),
                Output::new(always_succeed_address.clone(), value!(102049379)).with_datum_hash(
                    hash!("747a61e363e1fbee6a0ce234320a55bae7262ed62aa7e979d5d390339be3dd18"),
                ),
            ),
            (
                input!(
                    "0000000000000000000000000000000000000000000000000000000000000000",
                    1,
                ),
                output!(
                    "addr1vxu84ftxpzh3zd8p9awp2ytwzk5exj0fxcj7paur4kd4ytckt7nh9",
                    value!(10000000),
                ),
            ),
        ];

        // With a missing datum hash.
        let result = Transaction::build(
            &FIXTURE_PROTOCOL_PARAMETERS,
            &BTreeMap::from(resolved_inputs.clone()),
            |tx| {
                tx.with_inputs(vec![input!(
                    "d62db0b98b6df96645eec19d4728b385592fc531736abd987eb6490510c5ba50",
                    0,
                    PlutusData::list([]),
                )])
                .with_collaterals(vec![input!(
                    "0000000000000000000000000000000000000000000000000000000000000000",
                    1
                )])
                .with_change_strategy(ChangeStrategy::as_last_output(
                    address!("addr1qxu84ftxpzh3zd8p9awp2ytwzk5exj0fxcj7paur4kd4ytun36yuhgl049rxhhuckm2lpq3rmz5dcraddyl45d6xgvqqsp504c")
                ))
                .with_plutus_scripts(vec![always_succeed_script.clone()])
                .ok()
            },
        );

        let debug = format!("{result:#?}");

        assert!(
            result.is_err_and(|e| e.to_string().contains("MissingRequiredDatum")),
            "{debug}",
        );

        // Providing the datum hash
        let result = Transaction::build(
            &FIXTURE_PROTOCOL_PARAMETERS,
            &BTreeMap::from(resolved_inputs.clone()),
            |tx| {
                tx.with_inputs(vec![input!(
                    "d62db0b98b6df96645eec19d4728b385592fc531736abd987eb6490510c5ba50",
                    0,
                    PlutusData::list([]),
                )])
                .with_collaterals(vec![input!(
                    "0000000000000000000000000000000000000000000000000000000000000000",
                    1
                )])
                .with_change_strategy(ChangeStrategy::as_last_output(
                    address!("addr1qxu84ftxpzh3zd8p9awp2ytwzk5exj0fxcj7paur4kd4ytun36yuhgl049rxhhuckm2lpq3rmz5dcraddyl45d6xgvqqsp504c")
                ))
                .with_plutus_scripts(vec![always_succeed_script.clone()])
                .with_datums(vec![
                    plutus_data!(
                        "d8799fd8799fd87a9f581c1eae96baf29e27682ea3f815aba361a0c\
                         6059d45e4bfbe95bbd2f44affffd8799f4040ffd8799f581cf66d78\
                         b4a3cb3d37afa0ec36461e51ecbde00f26c8f0a68f94b6988044694\
                         55448ff1a9deac9cb1b00000033accac2401a02311bec18641864d8\
                         799f190e52ffd87980ff"
                    )
                ])
                .ok()
            },
        );

        assert_eq!(
            result
                .map(|tx| tx.to_string())
                .unwrap_or_else(|e| e.to_string()),
            "Transaction (id = 3bd44ee7393607ab23ac97bc0928cce42edf7316195d301308fc346877d8a55d) { \
                inputs: [Input(d62db0b98b6df96645eec19d4728b385592fc531736abd987eb6490510c5ba50#0)], \
                outputs: [\
                    Output { \
                        address: addr1qxu84ftxpzh3zd8p9awp2ytwzk5exj0fxcj7paur4kd4ytun36yuhgl049rxhhuckm2lpq3rmz5dcraddyl45d6xgvqqsp504c, \
                        value: Value { lovelace: 101870870 } \
                    }\
                ], \
                fee: 178509, \
                script_integrity_hash: 3b2ff5d0ea6d2fa720d12f01d71e015306d77524c750df84b2106bbe0919a4e2, \
                collaterals: [Input(0000000000000000000000000000000000000000000000000000000000000000#1)], \
                collateral_return: Output { address: addr1vxu84ftxpzh3zd8p9awp2ytwzk5exj0fxcj7paur4kd4ytckt7nh9, value: Value { lovelace: 9732236 } }, \
                total_collateral: 267764, \
                scripts: [v3(bd3ae991b5aafccafe5ca70758bd36a9b2f872f57f6d3a1ffa0eb777)], \
                datums: [\
                    CBOR(d8799fd8799fd87a9f581c1eae96baf29e27682ea3f815aba361a0c\
                         6059d45e4bfbe95bbd2f44affffd8799f4040ffd8799f581cf66d78\
                         b4a3cb3d37afa0ec36461e51ecbde00f26c8f0a68f94b6988044694\
                         55448ff1a9deac9cb1b00000033accac2401a02311bec18641864d8\
                         799f190e52ffd87980ff)\
                ], \
                redeemers: {\
                    Spend(0): Redeemer(CBOR(80), ExecutionUnits { mem: 1601, cpu: 316149 })\
                } \
            }"
        );
    }

    #[test]
    fn min_lovelace_value_with_nft() {
        let resolved_inputs = [(
            input!(
                "d62db0b98b6df96645eec19d4728b385592fc531736abd987eb6490510c5ba50",
                0
            ),
            output!(
                "addr1qxu84ftxpzh3zd8p9awp2ytwzk5exj0fxcj7paur4kd4ytun36yuhgl049rxhhuckm2lpq3rmz5dcraddyl45d6xgvqqsp504c",
                value!(
                    102049379,
                    (
                        "279c909f348e533da5808898f87f9a14bb2c3dfbbacccd631d927a3f",
                        "534e454b",
                        1
                    ),
                )
            ),
        )];

        let result = Transaction::build(
            &FIXTURE_PROTOCOL_PARAMETERS,
            &BTreeMap::from(resolved_inputs.clone()),
            |tx| {
                tx.with_inputs(vec![input!(
                    "d62db0b98b6df96645eec19d4728b385592fc531736abd987eb6490510c5ba50",
                    0,
                    _
                )])
                .with_outputs(vec![
                    output!("addr1qxu84ftxpzh3zd8p9awp2ytwzk5exj0fxcj7paur4kd4ytun36yuhgl049rxhhuckm2lpq3rmz5dcraddyl45d6xgvqqsp504c").with_assets(
                        assets!(
                            (
                                "279c909f348e533da5808898f87f9a14bb2c3dfbbacccd631d927a3f",
                                "534e454b",
                                1,
                            )
                        ),
                    ),
                ])
                .with_change_strategy(ChangeStrategy::as_last_output(
                    address!("addr1qxu84ftxpzh3zd8p9awp2ytwzk5exj0fxcj7paur4kd4ytun36yuhgl049rxhhuckm2lpq3rmz5dcraddyl45d6xgvqqsp504c")
                ))
                .ok()
            },
        );

        assert!(result.is_ok(), "{result:#?}");
    }

    #[test]
    fn full_lifecycle() {
        let always_succeed_script = ALWAYS_SUCCEED_SCRIPT;
        let always_succeed_address = ALWAYS_SUCCEED_ADDRESS;

        let my_address: Address<Any> = address_test!(
            "addr_test1qzpvzu5atl2yzf9x4eetekuxkm5z02kx5apsreqq8syjum6274ase8lkeffp39narear74ed0nf804e5drfm9l99v4eq3ecz8t"
        );

        let mut resolved_inputs = BTreeMap::from([(
            input!(
                "c984c8bf52a141254c714c905b2d27b432d4b546f815fbc2fea7b9da6e490324",
                3
            ),
            Output::new(my_address.clone(), value!(11875000)),
        )]);

        // https://preprod.cardanoscan.io/transaction/036fd8d808d4a87737cbb0ed1e61b08ce753323e94fc118c5eefabee6a8e04a5
        let deploy_script =
            Transaction::build(&FIXTURE_PROTOCOL_PARAMETERS, &resolved_inputs, |tx| {
                tx.with_inputs(vec![input!(
                    "c984c8bf52a141254c714c905b2d27b432d4b546f815fbc2fea7b9da6e490324",
                    3,
                    _
                )])
                .with_outputs(vec![
                    Output::to(my_address.clone())
                        .with_plutus_script(always_succeed_script.clone()),
                ])
                .with_change_strategy(ChangeStrategy::as_last_output(my_address.clone()))
                .ok()
            })
            .unwrap();

        assert_eq!(
            hex::encode(deploy_script.to_cbor()),
            "84a300d9010281825820c984c8bf52a141254c714c905b2d27b432d4b546f815fbc\
             2fea7b9da6e490324030182a30058390082c1729d5fd44124a6ae72bcdb86b6e827\
             aac6a74301e4003c092e6f4af57b0c9ff6ca5218967d1e7a3f572d7cd277d73468d\
             3b2fca56572011a001092a803d818558203525101010023259800a518a4d1365640\
             04ae69a20058390082c1729d5fd44124a6ae72bcdb86b6e827aac6a74301e4003c0\
             92e6f4af57b0c9ff6ca5218967d1e7a3f572d7cd277d73468d3b2fca56572011a00\
             a208bb021a00029755a0f5f6",
            "deploy_script no longer matches expected bytes."
        );

        resolved_inputs.append(&mut deploy_script.as_resolved_inputs());

        // https://preprod.cardanoscan.io/transaction/8d56891b4638203175c488e19d630bfbc8af285353aeeb1053d54a3c371b7a40
        let pay_to_script =
            Transaction::build(&FIXTURE_PROTOCOL_PARAMETERS, &resolved_inputs, |tx| {
                tx.with_inputs(vec![(Input::new(deploy_script.id(), 1), None)])
                    .with_outputs(vec![Output::new(
                        always_succeed_address.clone(),
                        value!(5_000_000),
                    )])
                    .with_change_strategy(ChangeStrategy::as_last_output(my_address.clone()))
                    .ok()
            })
            .unwrap();

        assert_eq!(
            hex::encode(pay_to_script.to_cbor()),
            "84a300d9010281825820036fd8d808d4a87737cbb0ed1e61b08ce753323e94fc118\
             c5eefabee6a8e04a5010182a200581d70bd3ae991b5aafccafe5ca70758bd36a9b2\
             f872f57f6d3a1ffa0eb777011a004c4b40a20058390082c1729d5fd44124a6ae72b\
             cdb86b6e827aac6a74301e4003c092e6f4af57b0c9ff6ca5218967d1e7a3f572d7c\
             d277d73468d3b2fca56572011a00532f42021a00028e39a0f5f6",
            "pay_to_script no longer matches expected bytes."
        );

        resolved_inputs.append(&mut pay_to_script.as_resolved_inputs());

        // https://preprod.cardanoscan.io/transaction/3522a630e91e631f56897be2898e059478c300f4bb8dd7891549a191b4bf1090
        let spend_from_script =
            Transaction::build(&FIXTURE_PROTOCOL_PARAMETERS, &resolved_inputs, |tx| {
                tx.with_inputs(vec![(
                    Input::new(pay_to_script.id(), 0),
                    Some(PlutusData::list(vec![])),
                )])
                .with_reference_inputs(vec![(Input::new(deploy_script.id(), 0))])
                .with_collaterals(vec![Input::new(pay_to_script.id(), 1)])
                .with_change_strategy(ChangeStrategy::as_last_output(
                    always_succeed_address.clone(),
                ))
                .ok()
            })
            .unwrap();

        assert_eq!(
            hex::encode(spend_from_script.to_cbor()),
            "84a800d90102818258208d56891b4638203175c488e19d630bfbc8af285353aeeb1\
             053d54a3c371b7a40000181a200581d70bd3ae991b5aafccafe5ca70758bd36a9b2\
             f872f57f6d3a1ffa0eb777011a0049a375021a0002a7cb0b5820d545623b07e425a\
             55262585d2b5e8aaee16230fd1434e790fa4511da4bf8a4550dd90102818258208d\
             56891b4638203175c488e19d630bfbc8af285353aeeb1053d54a3c371b7a400110a\
             20058390082c1729d5fd44124a6ae72bcdb86b6e827aac6a74301e4003c092e6f4a\
             f57b0c9ff6ca5218967d1e7a3f572d7cd277d73468d3b2fca56572011a004f33911\
             11a0003fbb112d9010281825820036fd8d808d4a87737cbb0ed1e61b08ce753323e\
             94fc118c5eefabee6a8e04a500a105a18200008280821906411a0004d2f5f5f6",
            "spend_from_script no longer matches expected bytes."
        );

        resolved_inputs.append(&mut spend_from_script.as_resolved_inputs());

        // https://preprod.cardanoscan.io/transaction/cd8c5bf00ab490d57c82ebf6364e4a6337dc214d635e8c392deaa7e4b98ed6ea
        let unpublish_script =
            Transaction::build(&FIXTURE_PROTOCOL_PARAMETERS, &resolved_inputs, |tx| {
                tx.with_inputs(vec![
                    (
                        Input::new(spend_from_script.id(), 0),
                        Some(PlutusData::list(vec![])),
                    ),
                    (Input::new(deploy_script.id(), 0), None),
                    (Input::new(pay_to_script.id(), 1), None),
                ])
                .with_collaterals(vec![Input::new(pay_to_script.id(), 1)])
                .with_change_strategy(ChangeStrategy::as_last_output(my_address.clone()))
                .ok()
            })
            .unwrap();

        assert_eq!(
            hex::encode(unpublish_script.to_cbor()),
            "84a700d9010283825820036fd8d808d4a87737cbb0ed1e61b08ce753323e94fc118\
             c5eefabee6a8e04a5008258203522a630e91e631f56897be2898e059478c300f4bb\
             8dd7891549a191b4bf1090008258208d56891b4638203175c488e19d630bfbc8af2\
             85353aeeb1053d54a3c371b7a40010181a20058390082c1729d5fd44124a6ae72bc\
             db86b6e827aac6a74301e4003c092e6f4af57b0c9ff6ca5218967d1e7a3f572d7cd\
             277d73468d3b2fca56572011a00aab370021a0002b1ef0b5820d37acc9c984616d9\
             d15825afeaf7d266e5bde38fdd4df4f8b2312703022d474d0dd90102818258208d5\
             6891b4638203175c488e19d630bfbc8af285353aeeb1053d54a3c371b7a400110a2\
             0058390082c1729d5fd44124a6ae72bcdb86b6e827aac6a74301e4003c092e6f4af\
             57b0c9ff6ca5218967d1e7a3f572d7cd277d73468d3b2fca56572011a004f245b11\
             1a00040ae7a105a18200018280821906411a0004d2f5f5f6",
            "unpublish_script no longer matches expected bytes."
        );
    }
}
