//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{
    Address, ChangeStrategy, ExecutionUnits, Hash, Input, Output, PlutusData, PlutusScript,
    PlutusVersion, ProtocolParameters, RedeemerPointer, Value, cbor, pallas,
};
use anyhow::anyhow;
use itertools::Itertools;
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fmt, iter, mem,
    ops::Deref,
};

mod builder;

pub struct Transaction {
    inner: pallas::Tx,
    change_strategy: ChangeStrategy,
}

type BoxedIterator<'iter, T> = Box<dyn Iterator<Item = T> + 'iter>;

// ------------------------------------------------------------------ Inspecting

impl fmt::Debug for Transaction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl Transaction {
    pub fn id(&self) -> Hash<32> {
        let mut bytes = Vec::new();
        let _ = cbor::encode(&self.inner.transaction_body, &mut bytes);
        Hash::from(pallas::hash::Hasher::<256>::hash(&bytes))
    }

    pub fn fee(&self) -> u64 {
        self.inner.transaction_body.fee
    }

    pub fn total_collateral(&self) -> u64 {
        self.inner
            .transaction_body
            .total_collateral
            .unwrap_or_default()
    }

    /// The declared transaction inputs, which are spent in case of successful transaction.
    pub fn inputs(&self) -> Box<dyn Iterator<Item = Input<'_>> + '_> {
        Box::new(
            self.inner
                .transaction_body
                .inputs
                .deref()
                .iter()
                .map(Input::from),
        )
    }

    /// The declared transaction collaterals, which are spent in case of failed transaction.
    pub fn collaterals(&self) -> Box<dyn Iterator<Item = Input<'_>> + '_> {
        self.inner
            .transaction_body
            .collateral
            .as_ref()
            .map(|xs| Box::new(xs.iter().map(Input::from)) as BoxedIterator<'_, Input<'_>>)
            .unwrap_or_else(|| Box::new(iter::empty()) as BoxedIterator<'_, Input<'_>>)
    }

    /// The declared transaction reference inputs, which are never spent but contribute to the
    /// script context for smart-contract execution.
    pub fn reference_inputs(&self) -> Box<dyn Iterator<Item = Input<'_>> + '_> {
        self.inner
            .transaction_body
            .reference_inputs
            .as_ref()
            .map(|xs| Box::new(xs.iter().map(Input::from)) as BoxedIterator<'_, Input<'_>>)
            .unwrap_or_else(|| Box::new(iter::empty()) as BoxedIterator<'_, Input<'_>>)
    }

    pub fn mint(&self) -> Value<i64> {
        self.inner
            .transaction_body
            .mint
            .as_ref()
            .map(Value::from)
            .unwrap_or_default()
    }

    /// The declared transaction outputs, which are produced in case of successful transaction.
    pub fn outputs(&self) -> Box<dyn Iterator<Item = Output<'_>> + '_> {
        Box::new(
            self.inner
                .transaction_body
                .outputs
                .iter()
                .cloned()
                .map(Output::try_from)
                .collect::<Result<Vec<_>, _>>()
                .expect("transaction contains invalid outputs; should be impossible at this point.")
                .into_iter(),
        )
    }

    /// View this transaction as a UTxO, mapping each output to its corresponding input reference.
    pub fn as_resolved_inputs(&self) -> BTreeMap<Input<'_>, Output<'_>> {
        let id = self.id();
        self.outputs()
            .enumerate()
            .fold(BTreeMap::new(), |mut resolved_inputs, (ix, output)| {
                resolved_inputs.insert(Input::new(id, ix as u64), output);
                resolved_inputs
            })
    }
}

// -------------------------------------------------------------------- Building

impl Default for Transaction {
    fn default() -> Self {
        Self {
            change_strategy: ChangeStrategy::default(),
            inner: pallas::Tx {
                transaction_body: pallas::TransactionBody {
                    auxiliary_data_hash: None,
                    certificates: None,
                    collateral: None,
                    collateral_return: None,
                    donation: None,
                    fee: 0,
                    inputs: pallas::Set::from(vec![]),
                    mint: None,
                    network_id: None,
                    outputs: vec![],
                    proposal_procedures: None,
                    reference_inputs: None,
                    required_signers: None,
                    script_data_hash: None,
                    total_collateral: None,
                    treasury_value: None,
                    ttl: None,
                    validity_interval_start: None,
                    voting_procedures: None,
                    withdrawals: None,
                },
                transaction_witness_set: pallas::WitnessSet {
                    bootstrap_witness: None,
                    native_script: None,
                    plutus_data: None,
                    plutus_v1_script: None,
                    plutus_v2_script: None,
                    plutus_v3_script: None,
                    redeemer: None,
                    vkeywitness: None,
                },
                success: true,
                auxiliary_data: pallas::Nullable::Null,
            },
        }
    }
}

impl Transaction {
    pub fn ok(&mut self) -> anyhow::Result<&mut Self> {
        Ok(self)
    }

    pub fn with_inputs<'a>(
        &mut self,
        inputs: impl IntoIterator<Item = (Input<'a>, Option<PlutusData>)>,
    ) -> &mut Self {
        let mut redeemers = BTreeMap::new();

        self.inner.transaction_body.inputs = pallas::Set::from(
            inputs
                .into_iter()
                .sorted()
                .enumerate()
                .map(|(ix, (input, redeemer))| {
                    if let Some(data) = redeemer {
                        redeemers.insert(RedeemerPointer::spend(ix as u32), data);
                    }

                    pallas::TransactionInput::from(input)
                })
                .collect::<Vec<_>>(),
        );

        self.with_redeemers(|tag| matches!(tag, pallas::RedeemerTag::Spend), redeemers);

        self
    }

    pub fn with_collaterals<'a>(
        &mut self,
        collaterals: impl IntoIterator<Item = Input<'a>>,
    ) -> &mut Self {
        self.inner.transaction_body.collateral = pallas::NonEmptySet::from_vec(
            collaterals
                .into_iter()
                .sorted()
                .map(pallas::TransactionInput::from)
                .collect::<Vec<_>>(),
        );
        self
    }

    pub fn with_reference_inputs<'a>(
        &mut self,
        reference_inputs: impl IntoIterator<Item = Input<'a>>,
    ) -> &mut Self {
        self.inner.transaction_body.reference_inputs = pallas::NonEmptySet::from_vec(
            reference_inputs
                .into_iter()
                .sorted()
                .map(pallas::TransactionInput::from)
                .collect::<Vec<_>>(),
        );
        self
    }

    pub fn with_outputs<'a>(&mut self, outputs: impl IntoIterator<Item = Output<'a>>) -> &mut Self {
        self.inner.transaction_body.outputs = outputs
            .into_iter()
            .map(pallas::TransactionOutput::from)
            .collect::<Vec<_>>();
        self
    }

    pub fn with_change_strategy(&mut self, with: ChangeStrategy) -> &mut Self {
        self.change_strategy = with;
        self
    }

    pub fn with_mint(
        &mut self,
        mint: BTreeMap<(Hash<28>, PlutusData), BTreeMap<Vec<u8>, i64>>,
    ) -> &mut Self {
        let (redeemers, mint) = mint.into_iter().enumerate().fold(
            (BTreeMap::new(), BTreeMap::new()),
            |(mut redeemers, mut mint), (index, ((policy, data), assets))| {
                mint.insert(policy, assets);

                redeemers.insert(RedeemerPointer::mint(index as u32), data);

                (redeemers, mint)
            },
        );

        let value = Value::default().with_assets(mint);

        self.inner.transaction_body.mint = <Option<pallas::Multiasset<_>>>::from(&value);

        self.with_redeemers(|tag| matches!(tag, pallas::RedeemerTag::Mint), redeemers);

        self
    }

    pub fn with_fee(&mut self, fee: u64) -> &mut Self {
        self.inner.transaction_body.fee = fee;
        self
    }

    pub fn with_plutus_scripts(
        &mut self,
        scripts: impl IntoIterator<Item = PlutusScript>,
    ) -> &mut Self {
        let (v1, v2, v3) = scripts.into_iter().fold(
            (vec![], vec![], vec![]),
            |(mut v1, mut v2, mut v3), script| {
                match script.version() {
                    PlutusVersion::V1 => {
                        if let Ok(v1_script) = <pallas::PlutusScript<1>>::try_from(script) {
                            v1.push(v1_script)
                        }
                    }
                    PlutusVersion::V2 => {
                        if let Ok(v2_script) = <pallas::PlutusScript<2>>::try_from(script) {
                            v2.push(v2_script)
                        }
                    }
                    PlutusVersion::V3 => {
                        if let Ok(v3_script) = <pallas::PlutusScript<3>>::try_from(script) {
                            v3.push(v3_script)
                        }
                    }
                };

                (v1, v2, v3)
            },
        );

        assert!(
            v1.is_empty(),
            "trying to set some Plutus V1 scripts; these aren't supported yet and may fail later down the builder.",
        );

        assert!(
            v2.is_empty(),
            "trying to set some Plutus V2 scripts; these aren't supported yet and may fail later down the builder.",
        );

        self.inner.transaction_witness_set.plutus_v1_script = pallas::NonEmptySet::from_vec(v1);
        self.inner.transaction_witness_set.plutus_v2_script = pallas::NonEmptySet::from_vec(v2);
        self.inner.transaction_witness_set.plutus_v3_script = pallas::NonEmptySet::from_vec(v3);

        self
    }
}

// -------------------------------------------------------------------- Internal

impl Transaction {
    /// The list of signatories explicitly listed in the transaction body, and visible to any
    /// underlying validator script. This is necessary a subset of the all signatories but the
    /// total set of inferred signatories may be larger due do transaction inputs.
    ///
    /// In the wild, this may also be called:
    ///
    /// - 'required_signers' (e.g. in the 'official' CDDL: <https://github.com/IntersectMBO/cardano-ledger/blob/232511b0fa01cd848cd7a569d1acc322124cf9b8/eras/conway/impl/cddl-files/conway.cddl#L142>)
    /// - 'extra_signatories' (e.g. in Aiken's stdlib: https://aiken-lang.github.io/stdlib/cardano/transaction.html#Transaction
    ///
    fn specified_signatories(&self) -> Box<dyn Iterator<Item = Hash<28>> + '_> {
        self.inner
            .transaction_body
            .required_signers
            .as_ref()
            .map(|xs| Box::new(xs.deref().iter().map(<Hash<_>>::from)) as BoxedIterator<'_, _>)
            .unwrap_or_else(|| Box::new(iter::empty()) as BoxedIterator<'_, _>)
    }

    /// The list of required signatories on the transaction, solely inferred from inputs,
    /// collaterals and explicitly specified signers.
    ///
    /// FIXME:
    /// - account for signers from native scripts
    /// - account for signers from certificates
    /// - account for signers from votes & proposals
    /// - account for signers from withdrawals
    fn required_signatories(
        &self,
        resolved_inputs: &BTreeMap<Input<'_>, Output<'_>>,
    ) -> anyhow::Result<BTreeSet<Hash<28>>> {
        Ok(self
            .specified_signatories()
            .chain(
                self.inputs()
                    .chain(self.collaterals())
                    .map(|input| {
                        let output =
                            resolved_inputs
                                .get(&input)
                                .ok_or(anyhow!("unknown = {input:?}").context(
                                    "unknown output for specified input or collateral input; found in transaction but not provided in resolved set",
                                ))?;
                        Ok::<_, anyhow::Error>(output)
                    })
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .filter_map(|output| {
                        let address = output.address();
                        let address = address.as_shelley()?;
                        address.payment_credential().as_verification_key()
                    }),
            )
            .collect::<BTreeSet<_>>())
    }

    /// The set of scripts that must be executed and pass for the transaction to be valid. These
    /// are specifically relevant to someone building a transaction as they each require a specific
    /// redeemer and each have execution costs.
    ///
    /// Those scripts are distinct from all scripts available in the transaction since one may
    /// introduce scripts via reference inputs or via the witness set, even if their (valid)
    /// execution isn't required.
    ///
    /// 'required_scripts' may, therefore, come from multiple sources:
    ///
    /// - inputs (for script-locked inputs)
    /// - certificates (for script-based credentials)
    /// - mint (for minting/burning policies)
    /// - withdrawals (for script-based credentials)
    /// - proposals (for any defined constitution guardrails)
    /// - votes (for script-based credentials)
    ///
    /// FIXME: the function is currently partial, as certificates, withdrawals, votes and proposals
    /// aren't implemented.
    fn required_scripts<'i, 'o>(
        &self,
        resolved_inputs: &BTreeMap<Input<'i>, Output<'o>>,
    ) -> BTreeMap<RedeemerPointer, Hash<28>> {
        let from_inputs = self
            .inputs()
            .enumerate()
            .filter_map(|(index, input)| Some((index, resolved_inputs.get(&input)?)))
            .filter_map(|(index, output)| {
                let payment_credential = output.address().as_shelley()?.payment_credential();
                Some((index, payment_credential.as_script()?))
            })
            .map(|(index, hash)| (RedeemerPointer::spend(index as u32), hash));

        let from_mint = self
            .inner
            .transaction_body
            .mint
            .as_ref()
            .map(|assets| {
                Box::new(assets.iter().enumerate().map(|(index, (policy, _))| {
                    (RedeemerPointer::mint(index as u32), Hash::from(policy))
                })) as Box<dyn Iterator<Item = (RedeemerPointer, Hash<28>)>>
            })
            .unwrap_or_else(|| {
                Box::new(std::iter::empty())
                    as Box<dyn Iterator<Item = (RedeemerPointer, Hash<28>)>>
            });

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
            body.voting_procedures.is_none(),
            "found votes in transaction: not supported yet",
        );

        assert!(
            body.proposal_procedures.is_none(),
            "found proposals in transaction: not supported yet",
        );

        std::iter::empty()
            .chain(from_inputs)
            .chain(from_mint)
            .collect()
    }

    /// Pre-condition: this assumes and only support Plutus V3.
    fn script_integrity_hash(&self, params: &ProtocolParameters) -> Option<Hash<32>> {
        let redeemers = self.inner.transaction_witness_set.redeemer.as_ref();

        let datums = self.inner.transaction_witness_set.plutus_data.as_ref();

        if redeemers.is_none() && datums.is_none() {
            return None;
        }

        let mut preimage: Vec<u8> = Vec::new();
        if let Some(redeemers) = redeemers {
            cbor::encode(redeemers, &mut preimage).unwrap();
        }

        if let Some(datums) = datums {
            cbor::encode(datums, &mut preimage).unwrap();
        }

        cbor::encode(
            pallas::NonEmptyKeyValuePairs::Def(vec![(
                PlutusVersion::V3,
                params.plutus_v3_cost_model(),
            )]),
            &mut preimage,
        )
        .unwrap();

        Some(Hash::from(pallas::hash::Hasher::<256>::hash(&preimage)))
    }

    fn with_change_output(&mut self, change: Value<u64>) -> anyhow::Result<()> {
        let min_change_value =
            Output::new(Address::default(), change.clone()).min_acceptable_value();

        if change.lovelace() < min_change_value {
            return Err(
                anyhow!("not enough funds to create a sufficiently large change output").context(
                    format!(
                        "current value={} lovelace, minimum required={}",
                        change.lovelace(),
                        min_change_value
                    ),
                ),
            );
        }

        let mut outputs = mem::take(&mut self.inner.transaction_body.outputs)
            .into_iter()
            .map(Output::try_from)
            .collect::<Result<VecDeque<_>, _>>()?;

        mem::take(&mut self.change_strategy).with(change, &mut outputs)?;

        self.with_outputs(outputs);

        Ok(())
    }

    fn with_redeemers(
        &mut self,
        discard_if: impl Fn(pallas::RedeemerTag) -> bool,
        redeemers: BTreeMap<RedeemerPointer, PlutusData>,
    ) -> &mut Self {
        let redeemers = into_pallas_redeemers(redeemers);

        let new_redeemers = if let Some(existing_redeemers) =
            mem::take(&mut self.inner.transaction_witness_set.redeemer)
        {
            let existing_redeemers = without_existing_redeemers(existing_redeemers, discard_if);
            Box::new(existing_redeemers.chain(redeemers))
                as Box<dyn Iterator<Item = (pallas::RedeemersKey, pallas::RedeemersValue)>>
        } else {
            Box::new(redeemers)
                as Box<dyn Iterator<Item = (pallas::RedeemersKey, pallas::RedeemersValue)>>
        };

        self.inner.transaction_witness_set.redeemer =
            pallas::NonEmptyKeyValuePairs::from_vec(new_redeemers.collect())
                .map(pallas::Redeemers::from);

        self
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
        self.inputs().try_fold(&mut change, |total_input, input| {
            let output = resolved_inputs.get(&input).ok_or_else(|| {
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
            .map(|input| {
                resolved_inputs.get(&input).ok_or_else(|| {
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
            let minimum_collateral = params.minimum_collateral(self.fee());

            total_collateral_value
                .checked_sub(&Value::new(minimum_collateral))
                .map_err(|e| e.context("insufficient collateral inputs"))?;

            self.inner.transaction_body.total_collateral = Some(minimum_collateral);
            self.inner.transaction_body.collateral_return = Some(pallas::TransactionOutput::from(
                // A bit misleading but 'total_collateral_value' now refers to the total amount brought
                // in, minus the the minimum required by the protocol, left out.
                Output::new(return_address, total_collateral_value),
            ));
        }

        Ok(())
    }
}

/// Obtain a more friendly representation of Pallas' redeemers, without any redeemer matching the
/// given predicate.
fn without_existing_redeemers(
    redeemers: pallas::Redeemers,
    predicate: impl Fn(pallas::RedeemerTag) -> bool,
) -> impl Iterator<Item = (pallas::RedeemersKey, pallas::RedeemersValue)> {
    match redeemers {
        pallas::Redeemers::List(..) => {
            unreachable!("found redeemers encoded as list: impossible with this library.")
        }
        pallas::Redeemers::Map(kv) => kv.into_iter().filter(move |(k, _)| !predicate(k.tag)),
    }
}

fn into_pallas_redeemers(
    redeemers: BTreeMap<RedeemerPointer, PlutusData>,
) -> impl Iterator<Item = (pallas::RedeemersKey, pallas::RedeemersValue)> {
    redeemers.into_iter().map(|(ptr, data)| {
        let key = pallas::RedeemersKey::from(ptr);

        let value = pallas::RedeemersValue {
            data: pallas::PlutusData::from(data),
            ex_units: pallas::ExUnits::from(ExecutionUnits::default()),
        };

        (key, value)
    })
}

// -------------------------------------------------------------------- Encoding

impl<C> cbor::Encode<C> for Transaction {
    fn encode<W: cbor::encode::write::Write>(
        &self,
        e: &mut cbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), cbor::encode::Error<W::Error>> {
        e.encode_with(&self.inner, ctx)?;
        Ok(())
    }
}

impl<'d, C> cbor::Decode<'d, C> for Transaction {
    fn decode(d: &mut cbor::Decoder<'d>, ctx: &mut C) -> Result<Self, cbor::decode::Error> {
        Ok(Self {
            inner: d.decode_with(ctx)?,
            change_strategy: ChangeStrategy::default(),
        })
    }
}
