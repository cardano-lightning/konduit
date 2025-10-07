//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{
    ExecutionUnits, Hash, Input, Output, PlutusData, PlutusScript, PlutusVersion,
    ProtocolParameters, RedeemerPointer, Value, cbor, pallas,
};
use itertools::Itertools;
use std::{collections::BTreeMap, ops::Deref};

mod builder;

#[derive(Debug, cbor::Encode, cbor::Decode)]
#[repr(transparent)]
#[cbor(transparent)]
pub struct Transaction(#[n(0)] pallas::Tx);

// ------------------------------------------------------------------ Inspecting

impl Transaction {
    pub fn fee(&self) -> u64 {
        self.0.transaction_body.fee
    }

    /// The declared transaction inputs, which are spent in case of successful transaction.
    pub fn inputs(&self) -> Vec<Input<'_>> {
        self.0
            .transaction_body
            .inputs
            .deref()
            .iter()
            .map(Input::from)
            .collect()
    }

    /// The declared transaction collaterals, which are spent in case of failed transaction.
    pub fn collaterals(&self) -> Vec<Input<'_>> {
        self.0
            .transaction_body
            .collateral
            .as_ref()
            .map(|xs| xs.iter().map(Input::from).collect())
            .unwrap_or_default()
    }

    /// The declared transaction outputs, which are produced in case of successful transaction.
    pub fn outputs(&self) -> Vec<Output<'_>> {
        self.0
            .transaction_body
            .outputs
            .iter()
            .map(Output::try_from)
            .collect::<Result<_, _>>()
            .expect("transaction contains invalid outputs; should be impossible at this point.")
    }

    /// The list of signatories explicitly listed in the transaction body, and visible to any
    /// underlying validator script. This is necessary a subset of the all signatories but the
    /// total set of inferred signatories may be larger due do transaction inputs.
    ///
    /// In the wild, this may also be called:
    ///
    /// - 'required_signers' (e.g. in the 'official' CDDL: <https://github.com/IntersectMBO/cardano-ledger/blob/232511b0fa01cd848cd7a569d1acc322124cf9b8/eras/conway/impl/cddl-files/conway.cddl#L142>)
    /// - 'extra_signatories' (e.g. in Aiken's stdlib: https://aiken-lang.github.io/stdlib/cardano/transaction.html#Transaction
    ///
    fn specified_signatories(&self) -> Vec<Hash<28>> {
        self.0
            .transaction_body
            .required_signers
            .as_ref()
            .map(|xs| xs.deref().iter().map(<Hash<_>>::from).collect())
            .unwrap_or_default()
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
        utxo: &BTreeMap<Input<'i>, Output<'o>>,
    ) -> BTreeMap<RedeemerPointer, Hash<28>> {
        let from_inputs = self
            .inputs()
            .into_iter()
            .enumerate()
            .filter_map(|(index, input)| Some((index, utxo.get(&input)?)))
            .filter_map(|(index, output)| {
                let payment_credential = output.address().as_shelley()?.payment_credential();
                Some((index, payment_credential.as_script()?))
            })
            .map(|(index, hash)| (RedeemerPointer::spend(index as u32), hash));

        let from_mint = self
            .0
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

        let body = &self.0.transaction_body;

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
        let redeemers = self.0.transaction_witness_set.redeemer.as_ref();

        let datums = self.0.transaction_witness_set.plutus_data.as_ref();

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
}

// -------------------------------------------------------------------- Building

impl Default for Transaction {
    fn default() -> Self {
        Self(pallas::Tx {
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
        })
    }
}

impl Transaction {
    pub fn ok(self) -> anyhow::Result<Self> {
        Ok(self)
    }

    pub fn with_inputs<'a>(mut self, inputs: impl IntoIterator<Item = Input<'a>>) -> Self {
        self.0.transaction_body.inputs = pallas::Set::from(
            inputs
                .into_iter()
                .sorted()
                .map(pallas::TransactionInput::from)
                .collect::<Vec<_>>(),
        );
        self
    }

    pub fn with_collaterals<'a>(
        mut self,
        collaterals: impl IntoIterator<Item = Input<'a>>,
    ) -> Self {
        self.0.transaction_body.collateral = pallas::NonEmptySet::from_vec(
            collaterals
                .into_iter()
                .sorted()
                .map(pallas::TransactionInput::from)
                .collect::<Vec<_>>(),
        );
        self
    }

    pub fn with_outputs<'a>(mut self, outputs: impl IntoIterator<Item = Output<'a>>) -> Self {
        self.0.transaction_body.outputs = outputs
            .into_iter()
            .map(pallas::TransactionOutput::from)
            .collect::<Vec<_>>();
        self
    }

    pub fn with_mint(
        mut self,
        mint: BTreeMap<(Hash<28>, PlutusData), BTreeMap<Vec<u8>, i64>>,
    ) -> Self {
        let (redeemers, mint) = mint.into_iter().enumerate().fold(
            (BTreeMap::new(), BTreeMap::new()),
            |(mut redeemers, mut mint), (index, ((policy, data), assets))| {
                mint.insert(policy, assets);

                redeemers.insert(RedeemerPointer::mint(index as u32), data);

                (redeemers, mint)
            },
        );

        let value = Value::default().with_assets(mint);

        self.0.transaction_body.mint = <Option<pallas::Multiasset<_>>>::from(&value);

        self.with_mint_redeemers(redeemers)
    }

    pub fn with_fee(mut self, fee: u64) -> Self {
        self.0.transaction_body.fee = fee;
        self
    }

    pub fn with_redeemers(
        mut self,
        redeemers: impl IntoIterator<
            Item = (
                Box<dyn Fn(&Self) -> anyhow::Result<RedeemerPointer>>,
                PlutusData,
            ),
        >,
    ) -> anyhow::Result<Self> {
        let resolved_redeemers = redeemers
            .into_iter()
            .map(|(lookup, data)| {
                let key = pallas::RedeemersKey::from(lookup(&self)?);

                let value = pallas::RedeemersValue {
                    data: pallas::PlutusData::from(data),
                    ex_units: pallas::ExUnits::from(ExecutionUnits::default()),
                };

                Ok((key, value))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        self.0.transaction_witness_set.redeemer =
            pallas::NonEmptyKeyValuePairs::from_vec(resolved_redeemers)
                .map(pallas::Redeemers::from);

        Ok(self)
    }

    pub fn with_plutus_scripts(mut self, scripts: impl IntoIterator<Item = PlutusScript>) -> Self {
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

        self.0.transaction_witness_set.plutus_v1_script = pallas::NonEmptySet::from_vec(v1);
        self.0.transaction_witness_set.plutus_v2_script = pallas::NonEmptySet::from_vec(v2);
        self.0.transaction_witness_set.plutus_v3_script = pallas::NonEmptySet::from_vec(v3);

        self
    }
}

// -------------------------------------------------------------------- Internal

impl Transaction {
    fn with_mint_redeemers(mut self, redeemers: BTreeMap<RedeemerPointer, PlutusData>) -> Self {
        let redeemers = into_pallas_redeemers(redeemers);

        let new_redeemers =
            if let Some(existing_redeemers) = self.0.transaction_witness_set.redeemer {
                let existing_redeemers = without_existing_redeemers(existing_redeemers, |tag| {
                    matches!(tag, pallas::RedeemerTag::Mint)
                });

                Box::new(existing_redeemers.chain(redeemers))
                    as Box<dyn Iterator<Item = (pallas::RedeemersKey, pallas::RedeemersValue)>>
            } else {
                Box::new(redeemers)
                    as Box<dyn Iterator<Item = (pallas::RedeemersKey, pallas::RedeemersValue)>>
            };

        self.0.transaction_witness_set.redeemer =
            pallas::NonEmptyKeyValuePairs::from_vec(new_redeemers.collect())
                .map(pallas::Redeemers::from);

        self
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
