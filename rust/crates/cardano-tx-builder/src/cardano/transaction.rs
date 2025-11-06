//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{
    Address, BoxedIterator, ChangeStrategy, ExecutionUnits, Hash, Input, NetworkId, Output,
    PlutusData, PlutusScript, PlutusVersion, ProtocolParameters, RedeemerPointer, SigningKey,
    SlotBound, Value, VerificationKey, cbor, pallas, pretty,
};
use anyhow::anyhow;
use itertools::Itertools;
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fmt, iter,
    marker::PhantomData,
    mem,
    ops::Deref,
};

mod builder;
pub mod state;
pub use state::IsTransactionBodyState;

/// A transaction, either under construction or fully signed.
///
/// The [`State`](IsTransactionBodyState) captures the current state of the transaction and
/// restricts the methods available based on the state. In practice, a transaction starts in the
/// [`state::InConstruction`] using either [`Self::default`] or provided in the callback to
/// [`Self::build`].
///
/// Then it reaches the [`state::ReadyForSigning`] which enables the method [`Self::sign`], and
/// forbid any method that modifies the transaction body.
///
/// Note that [`Self::build`] is currently the only way by which one can get a transaction in the
/// [`state::ReadyForSigning`].
pub struct Transaction<State: IsTransactionBodyState> {
    inner: pallas::Tx,
    change_strategy: ChangeStrategy,
    state: PhantomData<State>,
}

// -------------------------------------------------------------------- Building

impl Default for Transaction<state::InConstruction> {
    fn default() -> Self {
        Self {
            change_strategy: ChangeStrategy::default(),
            state: PhantomData,
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

impl Transaction<state::InConstruction> {
    pub fn ok(&mut self) -> anyhow::Result<&mut Self> {
        Ok(self)
    }

    pub fn with_inputs(
        &mut self,
        inputs: impl IntoIterator<Item = (Input, Option<PlutusData<'static>>)>,
    ) -> &mut Self {
        let mut redeemers = BTreeMap::new();

        self.inner.transaction_body.inputs = pallas::Set::from(
            inputs
                .into_iter()
                .sorted()
                .enumerate()
                .map(|(ix, (input, redeemer))| {
                    if let Some(data) = redeemer {
                        redeemers.insert(RedeemerPointer::from_spend(ix as u32), data);
                    }

                    pallas::TransactionInput::from(input)
                })
                .collect::<Vec<_>>(),
        );

        self.with_redeemers(|tag| matches!(tag, pallas::RedeemerTag::Spend), redeemers);

        self
    }

    pub fn with_collaterals(&mut self, collaterals: impl IntoIterator<Item = Input>) -> &mut Self {
        self.inner.transaction_body.collateral = pallas::NonEmptySet::from_vec(
            collaterals
                .into_iter()
                .sorted()
                .map(pallas::TransactionInput::from)
                .collect::<Vec<_>>(),
        );
        self
    }

    pub fn with_reference_inputs(
        &mut self,
        reference_inputs: impl IntoIterator<Item = Input>,
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

    pub fn with_specified_signatories(
        &mut self,
        verification_key_hashes: impl IntoIterator<Item = Hash<28>>,
    ) -> &mut Self {
        self.inner.transaction_body.required_signers = pallas::NonEmptySet::from_vec(
            verification_key_hashes
                .into_iter()
                .map(pallas::Hash::from)
                .collect(),
        );
        self
    }

    pub fn with_outputs(&mut self, outputs: impl IntoIterator<Item = Output>) -> &mut Self {
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
            |(mut redeemers, mut mint), (index, ((script_hash, data), assets))| {
                mint.insert(script_hash, assets);

                redeemers.insert(RedeemerPointer::from_mint(index as u32), data);

                (redeemers, mint)
            },
        );

        let value = Value::default().with_assets(mint);

        self.inner.transaction_body.mint = <Option<pallas::Multiasset<_>>>::from(&value);

        self.with_redeemers(|tag| matches!(tag, pallas::RedeemerTag::Mint), redeemers);

        self
    }

    pub fn with_validity_interval(&mut self, from: SlotBound, until: SlotBound) -> &mut Self {
        // In Conway, the lower-bound is *inclusive* while the upper-bound is *exclusive*; so we
        // must match what the user set to get the right serialisation.

        let from_inclusive = match from {
            SlotBound::None => None,
            SlotBound::Inclusive(bound) => Some(bound),
            SlotBound::Exclusive(bound) => Some(bound + 1),
        };

        let until_exclusive = match until {
            SlotBound::None => None,
            SlotBound::Inclusive(bound) => Some(bound + 1),
            SlotBound::Exclusive(bound) => Some(bound),
        };

        self.inner.transaction_body.validity_interval_start = from_inclusive;
        self.inner.transaction_body.ttl = until_exclusive;

        self
    }

    pub fn with_fee(&mut self, fee: u64) -> &mut Self {
        self.inner.transaction_body.fee = fee;
        self
    }

    pub fn with_datums(
        &mut self,
        datums: impl IntoIterator<Item = PlutusData<'static>>,
    ) -> &mut Self {
        self.inner.transaction_witness_set.plutus_data = pallas::NonEmptySet::from_vec(
            datums.into_iter().map(pallas::PlutusData::from).collect(),
        );

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

        debug_assert!(
            v1.is_empty(),
            "trying to set some Plutus V1 scripts; these aren't supported yet and may fail later down the builder.",
        );

        debug_assert!(
            v2.is_empty(),
            "trying to set some Plutus V2 scripts; these aren't supported yet and may fail later down the builder.",
        );

        self.inner.transaction_witness_set.plutus_v1_script = pallas::NonEmptySet::from_vec(v1);
        self.inner.transaction_witness_set.plutus_v2_script = pallas::NonEmptySet::from_vec(v2);
        self.inner.transaction_witness_set.plutus_v3_script = pallas::NonEmptySet::from_vec(v3);

        self
    }
}

// -------------------------------------------------------------------- Signing

impl Transaction<state::ReadyForSigning> {
    pub fn sign(&mut self, signing_key: SigningKey) -> &mut Self {
        let public_key = pallas::Bytes::from(Vec::from(<[u8; VerificationKey::SIZE]>::from(
            VerificationKey::from(&signing_key),
        )));

        let witness = pallas::VKeyWitness {
            vkey: public_key.clone(),
            signature: pallas::Bytes::from(Vec::from(signing_key.sign(self.id()).as_ref())),
        };

        if let Some(signatures) = mem::take(&mut self.inner.transaction_witness_set.vkeywitness) {
            // Unfortunately, we don't have a proper set at the Pallas level. We also don't want to
            // use an intermediate BTreeSet here because it would arbitrarily change the order of
            // witnesses (which do not matter, but may be confusing when indexing / browsing the
            // witnesses after the fact.
            //
            // So, we preserve the set by simply discarding any matching signature, should one
            // decide to sign again with the same key.
            self.inner.transaction_witness_set.vkeywitness = pallas::NonEmptySet::from_vec(
                signatures
                    .to_vec()
                    .into_iter()
                    .filter(|existing_witness| existing_witness.vkey != public_key)
                    .chain(vec![witness])
                    .collect(),
            );
        } else {
            self.inner.transaction_witness_set.vkeywitness =
                pallas::NonEmptySet::from_vec(vec![witness]);
        }

        self
    }
}

// ------------------------------------------------------------------ Inspecting

impl<State: IsTransactionBodyState> fmt::Debug for Transaction<State> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl<State: IsTransactionBodyState> fmt::Display for Transaction<State> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:#?}",
            pretty::Fmt(|f: &mut fmt::Formatter<'_>| {
                let mut debug_struct = f.debug_struct(&format!("Transaction (id = {})", self.id()));

                let body = &self.inner.transaction_body;

                if !body.inputs.is_empty() {
                    debug_struct.field(
                        "inputs",
                        &pretty::Fmt(|f: &mut fmt::Formatter<'_>| {
                            f.debug_list()
                                .entries(self.inputs().map(pretty::ViaDisplay))
                                .finish()
                        }),
                    );
                }

                if !body.outputs.is_empty() {
                    debug_struct.field(
                        "outputs",
                        &pretty::Fmt(|f: &mut fmt::Formatter<'_>| {
                            f.debug_list()
                                .entries(self.outputs().map(pretty::ViaDisplay))
                                .finish()
                        }),
                    );
                }

                debug_struct.field("fee", &self.fee());

                let valid_from = match body.validity_interval_start {
                    None => "]-∞".to_string(),
                    Some(i) => format!("[{i}"),
                };

                let valid_until = match body.ttl {
                    None => "+∞[".to_string(),
                    Some(i) => format!("{i}["),
                };

                debug_struct.field(
                    "validity",
                    &pretty::ViaDisplay(format!("{valid_from}; {valid_until}")),
                );

                debug_assert!(
                    body.certificates.is_none(),
                    "found certificates in transaction; not yet supported"
                );

                debug_assert!(
                    body.withdrawals.is_none(),
                    "found withdrawals in transaction; not yet supported"
                );

                debug_assert!(
                    body.auxiliary_data_hash.is_none(),
                    "found auxiliary_data_hash in transaction; not yet supported"
                );

                if body.mint.is_some() {
                    debug_struct.field("mint", &pretty::ViaDisplay(self.mint()));
                }

                if let Some(hash) = body.script_data_hash {
                    debug_struct.field(
                        "script_integrity_hash",
                        &pretty::ViaDisplay(Hash::from(hash)),
                    );
                }

                if body.collateral.is_some() {
                    debug_struct.field(
                        "collaterals",
                        &pretty::Fmt(|f: &mut fmt::Formatter<'_>| {
                            f.debug_list()
                                .entries(self.collaterals().map(pretty::ViaDisplay))
                                .finish()
                        }),
                    );
                }

                if body.required_signers.is_some() {
                    debug_struct.field(
                        "specified_signatories",
                        &pretty::Fmt(|f: &mut fmt::Formatter<'_>| {
                            f.debug_list()
                                .entries(self.specified_signatories().map(pretty::ViaDisplay))
                                .finish()
                        }),
                    );
                }

                if let Some(network_id) = body.network_id {
                    debug_struct.field(
                        "network_id",
                        &pretty::ViaDisplay(NetworkId::from(network_id)),
                    );
                }

                if let Some(collateral_return) = body
                    .collateral_return
                    .as_ref()
                    .and_then(|c| Output::try_from(c.clone()).ok())
                {
                    debug_struct.field("collateral_return", &pretty::ViaDisplay(collateral_return));
                }

                if let Some(total_collateral) = body.total_collateral {
                    debug_struct.field("total_collateral", &pretty::ViaDisplay(total_collateral));
                }

                if body.reference_inputs.is_some() {
                    debug_struct.field(
                        "reference_inputs",
                        &pretty::Fmt(|f: &mut fmt::Formatter<'_>| {
                            f.debug_list()
                                .entries(self.reference_inputs().map(pretty::ViaDisplay))
                                .finish()
                        }),
                    );
                }

                debug_assert!(
                    body.voting_procedures.is_none(),
                    "found votes in transaction; not yet supported"
                );

                debug_assert!(
                    body.proposal_procedures.is_none(),
                    "found proposals in transaction; not yet supported"
                );

                debug_assert!(
                    body.treasury_value.is_none(),
                    "found treasury value in transaction; not yet supported"
                );

                debug_assert!(
                    body.donation.is_none(),
                    "found treasury donation in transaction; not yet supported"
                );

                let witness_set = &self.inner.transaction_witness_set;

                if let Some(signatures) = &witness_set.vkeywitness {
                    debug_struct.field(
                        "signatures",
                        &pretty::Fmt(|f: &mut fmt::Formatter<'_>| {
                            let mut map = f.debug_map();

                            for witness in signatures.iter() {
                                map.entry(
                                    &pretty::ViaDisplay(hex::encode(&witness.vkey[..])),
                                    &pretty::ViaDisplay(hex::encode(&witness.signature[..])),
                                );
                            }

                            map.finish()
                        }),
                    );
                }

                debug_assert!(
                    witness_set.bootstrap_witness.is_none(),
                    "found bootstrap witness in transaction; not yet supported",
                );

                debug_assert!(
                    witness_set.native_script.is_none(),
                    "found native script in transaction; not yet supported",
                );

                if witness_set.plutus_v1_script.is_some()
                    || witness_set.plutus_v2_script.is_some()
                    || witness_set.plutus_v3_script.is_some()
                {
                    debug_struct.field(
                        "scripts",
                        &pretty::Fmt(|f: &mut fmt::Formatter<'_>| {
                            let v1_scripts = witness_set
                                .plutus_v1_script
                                .as_ref()
                                .map(|set| {
                                    Box::new(set.iter().cloned().map(PlutusScript::from))
                                        as BoxedIterator<PlutusScript>
                                })
                                .unwrap_or_else(|| {
                                    Box::new(iter::empty()) as BoxedIterator<PlutusScript>
                                });

                            let v2_scripts = witness_set
                                .plutus_v2_script
                                .as_ref()
                                .map(|set| {
                                    Box::new(set.iter().cloned().map(PlutusScript::from))
                                        as BoxedIterator<PlutusScript>
                                })
                                .unwrap_or_else(|| {
                                    Box::new(iter::empty()) as BoxedIterator<PlutusScript>
                                });

                            let v3_scripts = witness_set
                                .plutus_v3_script
                                .as_ref()
                                .map(|set| {
                                    Box::new(set.iter().cloned().map(PlutusScript::from))
                                        as BoxedIterator<PlutusScript>
                                })
                                .unwrap_or_else(|| {
                                    Box::new(iter::empty()) as BoxedIterator<PlutusScript>
                                });

                            let plutus_scripts = v1_scripts.chain(v2_scripts).chain(v3_scripts);

                            f.debug_list()
                                .entries(plutus_scripts.map(pretty::ViaDisplay))
                                .finish()
                        }),
                    );
                }

                if let Some(datums) = witness_set.plutus_data.as_ref() {
                    debug_struct.field(
                        "datums",
                        &pretty::Fmt(|f: &mut fmt::Formatter<'_>| {
                            f.debug_list()
                                .entries(
                                    datums
                                        .iter()
                                        .cloned()
                                        .map(PlutusData::from)
                                        .map(pretty::ViaDisplay),
                                )
                                .finish()
                        }),
                    );
                }

                if let Some(redeemers) = witness_set.redeemer.as_ref() {
                    debug_struct.field(
                        "redeemers",
                        &pretty::Fmt(|f: &mut fmt::Formatter<'_>| match redeemers {
                            pallas::Redeemers::List(_) => panic!(
                                "found redeemers encoded as list; shouldn't be possible with this builder."
                            ),
                            pallas::Redeemers::Map(map) => {
                                let mut redeemers = f.debug_map();
                                for (key, value) in map.iter() {
                                    redeemers.entry(
                                        &pretty::ViaDisplay(RedeemerPointer::from(key.clone())),
                                        &pretty::Fmt(|f: &mut fmt::Formatter<'_>| {
                                            f.debug_tuple("Redeemer")
                                                .field(&pretty::ViaDisplay(PlutusData::from(
                                                    value.data.clone(),
                                                )))
                                                .field(&pretty::ViaDisplay(ExecutionUnits::from(
                                                    value.ex_units,
                                                )))
                                                .finish()
                                        }),
                                    );
                                }
                                redeemers.finish()
                            }
                        }),
                    );
                }

                debug_struct.finish()
            })
        )
    }
}

impl<State: IsTransactionBodyState> Transaction<State> {
    /// The transaction identifier, as a _blake2b-256_ hash digest of its serialised body.
    ///
    /// <div class="warning">While the method can be called at any time, any change on the
    /// transaction body will alter the id. It is only stable when the state is
    /// [`state::ReadyForSigning`]</div>
    pub fn id(&self) -> Hash<32> {
        let mut bytes = Vec::new();
        let _ = cbor::encode(&self.inner.transaction_body, &mut bytes);
        Hash::from(pallas::Hasher::<256>::hash(&bytes))
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
    pub fn inputs(&self) -> Box<dyn Iterator<Item = Input> + '_> {
        Box::new(
            self.inner
                .transaction_body
                .inputs
                .deref()
                .iter()
                .cloned()
                .map(Input::from),
        )
    }

    /// The declared transaction collaterals, which are spent in case of failed transaction.
    pub fn collaterals(&self) -> Box<dyn Iterator<Item = Input> + '_> {
        self.inner
            .transaction_body
            .collateral
            .as_ref()
            .map(|xs| Box::new(xs.iter().cloned().map(Input::from)) as BoxedIterator<'_, Input>)
            .unwrap_or_else(|| Box::new(iter::empty()) as BoxedIterator<'_, Input>)
    }

    /// The declared transaction reference inputs, which are never spent but contribute to the
    /// script context for smart-contract execution.
    pub fn reference_inputs(&self) -> Box<dyn Iterator<Item = Input> + '_> {
        self.inner
            .transaction_body
            .reference_inputs
            .as_ref()
            .map(|xs| Box::new(xs.iter().cloned().map(Input::from)) as BoxedIterator<'_, Input>)
            .unwrap_or_else(|| Box::new(iter::empty()) as BoxedIterator<'_, Input>)
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
    pub fn outputs(&self) -> Box<dyn Iterator<Item = Output> + '_> {
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
    pub fn as_resolved_inputs(&self) -> BTreeMap<Input, Output> {
        let id = self.id();
        self.outputs()
            .enumerate()
            .fold(BTreeMap::new(), |mut resolved_inputs, (ix, output)| {
                resolved_inputs.insert(Input::new(id, ix as u64), output);
                resolved_inputs
            })
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
    fn specified_signatories(&self) -> Box<dyn Iterator<Item = Hash<28>> + '_> {
        self.inner
            .transaction_body
            .required_signers
            .as_ref()
            .map(|xs| Box::new(xs.deref().iter().map(<Hash<_>>::from)) as BoxedIterator<'_, _>)
            .unwrap_or_else(|| Box::new(iter::empty()) as BoxedIterator<'_, _>)
    }
}

// -------------------------------------------------------------------- Internal

impl<State: IsTransactionBodyState> Transaction<State> {
    /// The list of required signatories on the transaction, solely inferred from inputs,
    /// collaterals and explicitly specified signers.
    ///
    /// FIXME:
    ///
    /// - account for signers from native scripts
    /// - account for signers from certificates
    /// - account for signers from votes
    /// - account for signers from withdrawals
    fn required_signatories(
        &self,
        resolved_inputs: &BTreeMap<Input, Output>,
    ) -> anyhow::Result<BTreeSet<Hash<28>>> {
        let body = &self.inner.transaction_body;

        debug_assert!(
            body.certificates.is_none(),
            "found certificates in transaction: not supported yet",
        );

        debug_assert!(
            body.withdrawals.is_none(),
            "found withdrawals in transaction: not supported yet",
        );

        debug_assert!(
            body.voting_procedures.is_none(),
            "found votes in transaction: not supported yet",
        );

        Ok(self
            .specified_signatories()
            .chain(
                self.inputs()
                    .chain(self.collaterals())
                    .map(|input| {
                        let output =
                            resolved_inputs
                                .get(&input)
                                .ok_or(anyhow!("unknown = {input}").context(
                                    "unknown output for specified input or collateral input; found in transaction but not provided in resolved set",
                                ))?;
                        Ok::<_, anyhow::Error>(output)
                    })
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .filter_map(|output| {
                        let address = output.address();
                        let address = address.as_shelley()?;
                        address.payment().as_key()
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
    /// - mint (for minting/burning scripts)
    /// - withdrawals (for script-based credentials)
    /// - proposals (for any defined constitution guardrails)
    /// - votes (for script-based credentials)
    ///
    /// FIXME: the function is currently partial, as certificates, withdrawals, votes and proposals
    /// aren't implemented.
    fn required_scripts(
        &self,
        resolved_inputs: &BTreeMap<Input, Output>,
    ) -> BTreeMap<RedeemerPointer, Hash<28>> {
        let from_inputs = self
            .inputs()
            .enumerate()
            .filter_map(|(index, input)| Some((index, resolved_inputs.get(&input)?)))
            .filter_map(|(index, output)| {
                let payment_credential = output.address().as_shelley()?.payment();
                Some((index, payment_credential.as_script()?))
            })
            .map(|(index, hash)| (RedeemerPointer::from_spend(index as u32), hash));

        let from_mint = self
            .inner
            .transaction_body
            .mint
            .as_ref()
            .map(|assets| {
                Box::new(assets.iter().enumerate().map(|(index, (script_hash, _))| {
                    (
                        RedeemerPointer::from_mint(index as u32),
                        Hash::from(script_hash),
                    )
                })) as Box<dyn Iterator<Item = (RedeemerPointer, Hash<28>)>>
            })
            .unwrap_or_else(|| {
                Box::new(std::iter::empty())
                    as Box<dyn Iterator<Item = (RedeemerPointer, Hash<28>)>>
            });

        let body = &self.inner.transaction_body;

        debug_assert!(
            body.certificates.is_none(),
            "found certificates in transaction: not supported yet",
        );

        debug_assert!(
            body.withdrawals.is_none(),
            "found withdrawals in transaction: not supported yet",
        );

        debug_assert!(
            body.voting_procedures.is_none(),
            "found votes in transaction: not supported yet",
        );

        debug_assert!(
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
        debug_assert!(
            self.inner
                .transaction_witness_set
                .plutus_v1_script
                .is_none(),
            "found plutus v1 scripts in the transaction witness set; not supported yet"
        );

        debug_assert!(
            self.inner
                .transaction_witness_set
                .plutus_v2_script
                .is_none(),
            "found plutus v2 scripts in the transaction witness set; not supported yet"
        );

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

        Some(Hash::from(pallas::Hasher::<256>::hash(&preimage)))
    }
}

impl Transaction<state::InConstruction> {
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

        mem::take(&mut self.change_strategy).apply(change, &mut outputs)?;

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
            let mut err = anyhow!("required_scripts = {ptr} -> {hash}");
            for (ptr, hash) in scripts {
                err = err.context(format!("required_scripts = {ptr} -> {hash}"));
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
                anyhow!("extraneous redeemers in transaction; not required by any script").context(
                    format!(
                        "extra={:?}",
                        redeemers
                            .keys()
                            .map(|ptr| ptr.to_string())
                            .collect::<Vec<_>>()
                    ),
                ),
            );
        }

        Ok(())
    }

    fn with_change(&mut self, resolved_inputs: &BTreeMap<Input, Output>) -> anyhow::Result<()> {
        let mut change = Value::default();

        // Add inputs to the change balance
        self.inputs().try_fold(&mut change, |total_input, input| {
            let output = resolved_inputs.get(&input).ok_or_else(|| {
                anyhow!("unknown input, not present in resolved set")
                    .context(format!("input={input}"))
            })?;

            Ok::<_, anyhow::Error>(total_input.add(output.value()))
        })?;

        // Partition mint quantities between mint & burn
        let (mint, burn) = self.mint().assets().clone().into_iter().fold(
            (BTreeMap::new(), BTreeMap::new()),
            |(mut mint, mut burn), (script_hash, assets)| {
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
                    mint.insert(script_hash, minted_assets);
                }

                if !burned_assets.is_empty() {
                    burn.insert(script_hash, burned_assets);
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
                total_output.checked_sub(output.value())
            })
            .map_err(|e| e.context("insufficient balance; spending more than available"))?;

        // Subtract the transaction fee as well
        change
            .checked_sub(&Value::new(self.fee()))
            .map_err(|e| e.context("insufficient balance; spending more than available"))?;

        let body = &self.inner.transaction_body;

        debug_assert!(
            body.certificates.is_none(),
            "found certificates in transaction: not supported yet",
        );

        debug_assert!(
            body.withdrawals.is_none(),
            "found withdrawals in transaction: not supported yet",
        );

        debug_assert!(
            body.treasury_value.is_none(),
            "found treasury donation in transaction: not supported yet",
        );

        debug_assert!(
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
        resolved_inputs: &BTreeMap<Input, Output>,
        params: &ProtocolParameters,
    ) -> anyhow::Result<()> {
        let (mut total_collateral_value, opt_return_address): (Value<u64>, Option<Address<_>>) =
            self.collaterals()
                .map(|input| {
                    resolved_inputs.get(&input).ok_or_else(|| {
                        anyhow!("unknown collateral input").context(format!("reference={input}"))
                    })
                })
                .try_fold(
                    (Value::new(0), None),
                    |(mut total, address), maybe_output| {
                        let output = maybe_output?;
                        total.add(output.value());
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

impl<C, State: IsTransactionBodyState> cbor::Encode<C> for Transaction<State> {
    fn encode<W: cbor::encode::write::Write>(
        &self,
        e: &mut cbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), cbor::encode::Error<W::Error>> {
        e.encode_with(&self.inner, ctx)?;
        Ok(())
    }
}

impl<'d, C, State: IsTransactionBodyState> cbor::Decode<'d, C> for Transaction<State> {
    fn decode(d: &mut cbor::Decoder<'d>, ctx: &mut C) -> Result<Self, cbor::decode::Error> {
        Ok(Self {
            inner: d.decode_with(ctx)?,
            state: PhantomData,
            change_strategy: ChangeStrategy::default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{SigningKey, Transaction, cbor, transaction::state::*};
    use indoc::indoc;

    #[test]
    fn display_transaction_1() {
        let mut transaction: Transaction<ReadyForSigning> = cbor::decode(
            &hex::decode(
                "84a300d9010281825820c984c8bf52a141254c714c905b2d27b432d4b546f815fbc\
                 2fea7b9da6e490324030182a30058390082c1729d5fd44124a6ae72bcdb86b6e827\
                 aac6a74301e4003c092e6f4af57b0c9ff6ca5218967d1e7a3f572d7cd277d73468d\
                 3b2fca56572011a001092a803d818558203525101010023259800a518a4d1365640\
                 04ae69a20058390082c1729d5fd44124a6ae72bcdb86b6e827aac6a74301e4003c0\
                 92e6f4af57b0c9ff6ca5218967d1e7a3f572d7cd277d73468d3b2fca56572011a00\
                 a208bb021a00029755a0f5f6\
                ",
            )
            .unwrap(),
        )
        .unwrap();

        let signing_key = SigningKey::from([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        transaction.sign(signing_key);

        assert_eq!(
            transaction.to_string(),
            indoc! {"
                Transaction (id = 036fd8d808d4a87737cbb0ed1e61b08ce753323e94fc118c5eefabee6a8e04a5) {
                    inputs: [
                        Input(c984c8bf52a141254c714c905b2d27b432d4b546f815fbc2fea7b9da6e490324#3),
                    ],
                    outputs: [
                        Output {
                            address: addr_test1qzpvzu5atl2yzf9x4eetekuxkm5z02kx5apsreqq8syjum6274ase8lkeffp39narear74ed0nf804e5drfm9l99v4eq3ecz8t,
                            value: Value {
                                lovelace: 1086120,
                            },
                            script: v3(bd3ae991b5aafccafe5ca70758bd36a9b2f872f57f6d3a1ffa0eb777),
                        },
                        Output {
                            address: addr_test1qzpvzu5atl2yzf9x4eetekuxkm5z02kx5apsreqq8syjum6274ase8lkeffp39narear74ed0nf804e5drfm9l99v4eq3ecz8t,
                            value: Value {
                                lovelace: 10619067,
                            },
                        },
                    ],
                    fee: 169813,
                    validity: ]-∞; +∞[,
                    signatures: {
                        3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29: d739204915ea986ce309662cadfab44f8ffb9b0c10c6ade3839e2c5b11a6ba738ee2cbb1365ab714312fb79af0effb98c54ec92c88c99967e1e6cc87b56dc90e,
                    },
                }"
            },
        );
    }

    #[test]
    fn display_transaction_2() {
        let transaction: Transaction<ReadyForSigning> = cbor::decode(
            &hex::decode(
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
                 1a00040ae7a105a18200018280821906411a0004d2f5f5f6\
                ",
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(
            transaction.to_string(),
            indoc! {"
                Transaction (id = cd8c5bf00ab490d57c82ebf6364e4a6337dc214d635e8c392deaa7e4b98ed6ea) {
                    inputs: [
                        Input(036fd8d808d4a87737cbb0ed1e61b08ce753323e94fc118c5eefabee6a8e04a5#0),
                        Input(3522a630e91e631f56897be2898e059478c300f4bb8dd7891549a191b4bf1090#0),
                        Input(8d56891b4638203175c488e19d630bfbc8af285353aeeb1053d54a3c371b7a40#1),
                    ],
                    outputs: [
                        Output {
                            address: addr_test1qzpvzu5atl2yzf9x4eetekuxkm5z02kx5apsreqq8syjum6274ase8lkeffp39narear74ed0nf804e5drfm9l99v4eq3ecz8t,
                            value: Value {
                                lovelace: 11187056,
                            },
                        },
                    ],
                    fee: 176623,
                    validity: ]-∞; +∞[,
                    script_integrity_hash: d37acc9c984616d9d15825afeaf7d266e5bde38fdd4df4f8b2312703022d474d,
                    collaterals: [
                        Input(8d56891b4638203175c488e19d630bfbc8af285353aeeb1053d54a3c371b7a40#1),
                    ],
                    collateral_return: Output {
                        address: addr_test1qzpvzu5atl2yzf9x4eetekuxkm5z02kx5apsreqq8syjum6274ase8lkeffp39narear74ed0nf804e5drfm9l99v4eq3ecz8t,
                        value: Value {
                            lovelace: 5186651,
                        },
                    },
                    total_collateral: 264935,
                    redeemers: {
                        Spend(1): Redeemer(
                            CBOR(80),
                            ExecutionUnits {
                                mem: 1601,
                                cpu: 316149,
                            },
                        ),
                    },
                }"
            },
        );
    }
}
