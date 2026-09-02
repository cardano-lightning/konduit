use anyhow::{Context, anyhow};
use cardano_sdk::{Address, Hash, address::kind, cbor};
use pallas_primitives::conway::{
    MintedDatumOption, MintedScriptRef, MintedTransactionOutput, PlutusScript, PseudoDatumOption,
    PseudoScript, PseudoTransactionOutput, Value,
};
use std::collections::BTreeSet;
use utxorpc::spec::{cardano, query};

const MAX_INLINE_DATUM_BYTES: usize = 64 * 1024;
const MAX_SCRIPT_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappedAsset {
    pub unit: String,
    pub quantity: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappedUtxo {
    pub transaction_id: [u8; 32],
    pub output_index: u32,
    pub address: String,
    pub value: Vec<MappedAsset>,
    pub datum_hash: Option<[u8; 32]>,
    pub datum_inline: Option<Vec<u8>>,
    pub reference_script_hash: Option<[u8; 28]>,
    pub reference_script: Option<Vec<u8>>,
    pub reference_script_version: Option<u8>,
}

pub fn predicate_for_exact_address(address: &[u8]) -> query::UtxoPredicate {
    query::UtxoPredicate {
        r#match: Some(query::AnyUtxoPattern {
            utxo_pattern: Some(query::any_utxo_pattern::UtxoPattern::Cardano(
                cardano::TxOutputPattern {
                    address: Some(cardano::AddressPattern {
                        exact_address: address.to_vec().into(),
                        payment_part: Default::default(),
                        delegation_part: Default::default(),
                    }),
                    asset: None,
                },
            )),
        }),
        not: vec![],
        all_of: vec![],
        any_of: vec![],
    }
}

pub fn map_wire_utxo(utxo: utxorpc::ChainUtxo<cardano::TxOutput>) -> anyhow::Result<MappedUtxo> {
    let reference = utxo
        .txo_ref
        .ok_or_else(|| anyhow!("UTxO response missing txo_ref"))?;
    let transaction_id = bytes32(reference.hash.as_ref(), "tx hash")?;

    if !utxo.native.is_empty() {
        let minted: MintedTransactionOutput<'_> = cbor::decode(utxo.native.as_ref())
            .context("failed to decode native transaction output bytes")?;
        return mapped_from_minted(transaction_id, reference.index, minted);
    }

    let parsed = utxo
        .parsed
        .ok_or_else(|| anyhow!("UTxO response missing parsed output and native bytes"))?;
    mapped_from_parsed(transaction_id, reference.index, parsed)
}

fn mapped_from_minted(
    transaction_id: [u8; 32],
    output_index: u32,
    minted: MintedTransactionOutput<'_>,
) -> anyhow::Result<MappedUtxo> {
    match minted {
        PseudoTransactionOutput::Legacy(legacy) => {
            let amount = match legacy.amount {
                pallas_primitives::alonzo::Value::Coin(coin) => vec![MappedAsset {
                    unit: "lovelace".to_string(),
                    quantity: coin,
                }],
                pallas_primitives::alonzo::Value::Multiasset(coin, assets) => {
                    map_legacy_value(coin, &assets)?
                }
            };
            finish(
                transaction_id,
                output_index,
                legacy.address.as_ref(),
                amount,
                legacy.datum_hash.map(|hash| *hash),
                None,
                None,
            )
        }
        PseudoTransactionOutput::PostAlonzo(output) => {
            let (datum_hash, datum_inline) = match output.datum_option {
                Some(PseudoDatumOption::Hash(hash)) => (Some(*hash), None),
                Some(MintedDatumOption::Data(data)) => {
                    let raw = data.raw_cbor();
                    (None, Some(raw.to_vec()))
                }
                None => (None, None),
            };
            let script = output
                .script_ref
                .map(|wrap| map_minted_script(wrap.unwrap()))
                .transpose()?;
            finish(
                transaction_id,
                output_index,
                output.address.as_ref(),
                map_value(&output.value)?,
                datum_hash,
                datum_inline,
                script,
            )
        }
    }
}

fn mapped_from_parsed(
    transaction_id: [u8; 32],
    output_index: u32,
    parsed: cardano::TxOutput,
) -> anyhow::Result<MappedUtxo> {
    let address = Address::<kind::Any>::try_from(parsed.address.as_ref())?.to_string();
    let coin = parsed
        .coin
        .as_ref()
        .ok_or_else(|| anyhow!("parsed UTxO output missing coin value"))?;
    let mut value = vec![MappedAsset {
        unit: "lovelace".to_string(),
        quantity: crate::mapping::big_int_to_u64(coin)?,
    }];
    let mut seen = BTreeSet::from(["lovelace".to_string()]);

    for multiasset in parsed.assets {
        let policy = policy_unit(multiasset.policy_id.as_ref())?;
        for asset in multiasset.assets {
            let quantity = match &asset.quantity {
                Some(cardano::asset::Quantity::OutputCoin(quantity)) => {
                    crate::mapping::big_int_to_u64(quantity)?
                }
                Some(cardano::asset::Quantity::MintCoin(_)) => {
                    return Err(anyhow!("parsed UTxO asset used mint quantity"));
                }
                None => return Err(anyhow!("parsed asset missing quantity")),
            };
            let unit = format!("{policy}{}", hex::encode(asset.name.as_ref()));
            if !seen.insert(unit.clone()) {
                return Err(anyhow!("duplicate asset unit {unit}"));
            }
            value.push(MappedAsset { unit, quantity });
        }
    }

    let mut datum_hash = None;
    let mut datum_inline = None;
    if let Some(datum) = parsed.datum {
        if !datum.original_cbor.is_empty() {
            if datum.original_cbor.len() > MAX_INLINE_DATUM_BYTES {
                return Err(anyhow!("inline datum exceeds size bound"));
            }
            datum_inline = Some(datum.original_cbor.to_vec());
        }
        if !datum.hash.is_empty() {
            datum_hash = Some(bytes32(datum.hash.as_ref(), "datum hash")?);
        }
    }

    let (reference_script_hash, reference_script, reference_script_version) = match parsed.script {
        Some(script) => map_parsed_script(script)?,
        None => (None, None, None),
    };

    Ok(MappedUtxo {
        transaction_id,
        output_index,
        address,
        value,
        datum_hash,
        datum_inline,
        reference_script_hash,
        reference_script,
        reference_script_version,
    })
}

fn finish(
    transaction_id: [u8; 32],
    output_index: u32,
    address_bytes: &[u8],
    amount: Vec<MappedAsset>,
    datum_hash: Option<[u8; 32]>,
    datum_inline: Option<Vec<u8>>,
    script: Option<([u8; 28], Vec<u8>, u8)>,
) -> anyhow::Result<MappedUtxo> {
    if datum_inline
        .as_ref()
        .is_some_and(|bytes| bytes.len() > MAX_INLINE_DATUM_BYTES)
    {
        return Err(anyhow!("inline datum exceeds size bound"));
    }
    let address = Address::<kind::Any>::try_from(address_bytes)?.to_string();
    let (reference_script_hash, reference_script, reference_script_version) = match script {
        Some((hash, bytes, version)) => {
            if bytes.len() > MAX_SCRIPT_BYTES {
                return Err(anyhow!("reference script exceeds size bound"));
            }
            (Some(hash), Some(bytes), Some(version))
        }
        None => (None, None, None),
    };
    Ok(MappedUtxo {
        transaction_id,
        output_index,
        address,
        value: amount,
        datum_hash,
        datum_inline,
        reference_script_hash,
        reference_script,
        reference_script_version,
    })
}

fn map_value(value: &Value) -> anyhow::Result<Vec<MappedAsset>> {
    let (lovelace, assets) = match value {
        Value::Coin(coin) => (*coin, None),
        Value::Multiasset(coin, assets) => (*coin, Some(assets)),
    };
    let mut out = vec![MappedAsset {
        unit: "lovelace".to_string(),
        quantity: lovelace,
    }];
    let mut seen = BTreeSet::from(["lovelace".to_string()]);
    if let Some(assets) = assets {
        for (policy, names) in assets.iter() {
            let policy = policy_unit(policy.as_ref())?;
            for (name, quantity) in names.iter() {
                let unit = format!("{policy}{}", hex::encode(name.to_vec()));
                if !seen.insert(unit.clone()) {
                    return Err(anyhow!("duplicate asset unit {unit}"));
                }
                out.push(MappedAsset {
                    unit,
                    quantity: u64::from(*quantity),
                });
            }
        }
    }
    Ok(out)
}

fn map_legacy_value(
    lovelace: u64,
    assets: &pallas_primitives::alonzo::Multiasset<u64>,
) -> anyhow::Result<Vec<MappedAsset>> {
    let mut out = vec![MappedAsset {
        unit: "lovelace".to_string(),
        quantity: lovelace,
    }];
    let mut seen = BTreeSet::from(["lovelace".to_string()]);
    for (policy, names) in assets.iter() {
        let policy = policy_unit(policy.as_ref())?;
        for (name, quantity) in names.iter() {
            let unit = format!("{policy}{}", hex::encode(name.to_vec()));
            if !seen.insert(unit.clone()) {
                return Err(anyhow!("duplicate asset unit {unit}"));
            }
            out.push(MappedAsset {
                unit,
                quantity: *quantity,
            });
        }
    }
    Ok(out)
}

fn policy_unit(policy: &[u8]) -> anyhow::Result<String> {
    if policy.len() != 28 {
        return Err(anyhow!("unexpected policy id length: {}", policy.len()));
    }
    Ok(hex::encode(policy))
}

fn map_minted_script(script: MintedScriptRef<'_>) -> anyhow::Result<([u8; 28], Vec<u8>, u8)> {
    match script {
        PseudoScript::NativeScript(script) => {
            let cbor = script.raw_cbor().to_vec();
            Ok((script_hash(0, &cbor), cbor, 0))
        }
        PseudoScript::PlutusV1Script(PlutusScript(bytes)) => {
            let cbor = bytes.to_vec();
            Ok((script_hash(1, &cbor), cbor, 1))
        }
        PseudoScript::PlutusV2Script(PlutusScript(bytes)) => {
            let cbor = bytes.to_vec();
            Ok((script_hash(2, &cbor), cbor, 2))
        }
        PseudoScript::PlutusV3Script(PlutusScript(bytes)) => {
            let cbor = bytes.to_vec();
            Ok((script_hash(3, &cbor), cbor, 3))
        }
    }
}

fn map_parsed_script(
    script: cardano::Script,
) -> anyhow::Result<(Option<[u8; 28]>, Option<Vec<u8>>, Option<u8>)> {
    match script.script {
        Some(cardano::script::Script::Native(_)) => Err(anyhow!(
            "native reference script requires original output bytes"
        )),
        Some(cardano::script::Script::PlutusV1(bytes)) => ok_script(1, bytes.as_ref()),
        Some(cardano::script::Script::PlutusV2(bytes)) => ok_script(2, bytes.as_ref()),
        Some(cardano::script::Script::PlutusV3(bytes)) => ok_script(3, bytes.as_ref()),
        None => Err(anyhow!("script payload missing")),
    }
}

fn ok_script(
    version: u8,
    bytes: &[u8],
) -> anyhow::Result<(Option<[u8; 28]>, Option<Vec<u8>>, Option<u8>)> {
    if bytes.len() > MAX_SCRIPT_BYTES {
        return Err(anyhow!("reference script exceeds size bound"));
    }
    Ok((
        Some(script_hash(version, bytes)),
        Some(bytes.to_vec()),
        Some(version),
    ))
}

fn script_hash(language: u8, script: &[u8]) -> [u8; 28] {
    let mut preimage = Vec::with_capacity(1 + script.len());
    preimage.push(language);
    preimage.extend_from_slice(script);
    Hash::<28>::new(preimage).into()
}

fn bytes32(bytes: &[u8], label: &str) -> anyhow::Result<[u8; 32]> {
    bytes
        .try_into()
        .map_err(|_| anyhow!("unexpected {label} length: {}", bytes.len()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cardano_sdk::{address, cbor::ToCbor, key_credential};
    use pallas_codec::utils::CborWrap;
    use pallas_primitives::{
        KeyValuePairs,
        alonzo::{TransactionOutput as LegacyTransactionOutput, Value as LegacyValue},
        conway::{NativeScript, PostAlonzoTransactionOutput, ScriptRef, TransactionOutput},
    };
    use utxorpc::{ChainUtxo, NativeBytes};

    fn addr_bytes() -> Vec<u8> {
        let payment = key_credential!("11111111111111111111111111111111111111111111111111111111");
        Vec::from(&address!(payment))
    }

    #[test]
    fn native_reference_script_from_output_bytes() {
        let native = NativeScript::ScriptPubkey([0x22; 28].into());
        let mut script_cbor = Vec::new();
        cbor::encode(&native, &mut script_cbor).expect("encode native");
        let output = TransactionOutput::PostAlonzo(PostAlonzoTransactionOutput {
            address: addr_bytes().into(),
            value: Value::Coin(5_000_000),
            datum_option: None,
            script_ref: Some(CborWrap(ScriptRef::NativeScript(native))),
        });
        let native_bytes = output.to_cbor();
        let mapped = map_wire_utxo(ChainUtxo {
            parsed: None,
            native: NativeBytes::from(native_bytes),
            txo_ref: Some(query::TxoRef {
                hash: vec![0xab; 32].into(),
                index: 7,
            }),
        })
        .expect("native script utxo");

        assert_eq!(mapped.output_index, 7);
        assert_eq!(mapped.reference_script_version, Some(0));
        assert_eq!(
            mapped.reference_script.as_deref(),
            Some(script_cbor.as_slice())
        );
        assert_eq!(
            mapped.reference_script_hash,
            Some(script_hash(0, &script_cbor))
        );
        assert_eq!(mapped.value[0].quantity, 5_000_000);
    }

    #[test]
    fn legacy_multiasset_output_keeps_tokens() {
        let output = LegacyTransactionOutput {
            address: addr_bytes().into(),
            amount: LegacyValue::Multiasset(
                5_000_000,
                KeyValuePairs::from(vec![(
                    [0xab; 28].into(),
                    KeyValuePairs::from(vec![(vec![0xcd].into(), 42)]),
                )]),
            ),
            datum_hash: None,
        };
        let mapped = map_wire_utxo(ChainUtxo {
            parsed: None,
            native: NativeBytes::from(output.to_cbor()),
            txo_ref: Some(query::TxoRef {
                hash: vec![0x11; 32].into(),
                index: 0,
            }),
        })
        .expect("legacy multiasset utxo");

        assert_eq!(
            mapped.value,
            vec![
                MappedAsset {
                    unit: "lovelace".to_string(),
                    quantity: 5_000_000,
                },
                MappedAsset {
                    unit: format!("{}cd", "ab".repeat(28)),
                    quantity: 42,
                },
            ]
        );
    }

    #[test]
    fn datum_hash_only_from_parsed() {
        let mapped = map_wire_utxo(ChainUtxo {
            parsed: Some(cardano::TxOutput {
                address: addr_bytes().into(),
                coin: Some(cardano::BigInt {
                    big_int: Some(cardano::big_int::BigInt::Int(1)),
                }),
                datum: Some(cardano::Datum {
                    hash: vec![0xcd; 32].into(),
                    payload: None,
                    original_cbor: Default::default(),
                }),
                ..Default::default()
            }),
            native: NativeBytes::new(),
            txo_ref: Some(query::TxoRef {
                hash: vec![0x11; 32].into(),
                index: 0,
            }),
        })
        .expect("datum hash utxo");

        assert_eq!(mapped.datum_hash, Some([0xcd; 32]));
        assert!(mapped.datum_inline.is_none());
    }

    #[test]
    fn parsed_native_script_without_bytes_fails() {
        let error = map_wire_utxo(ChainUtxo {
            parsed: Some(cardano::TxOutput {
                address: addr_bytes().into(),
                coin: Some(cardano::BigInt {
                    big_int: Some(cardano::big_int::BigInt::Int(1)),
                }),
                script: Some(cardano::Script {
                    script: Some(cardano::script::Script::Native(cardano::NativeScript {
                        native_script: None,
                    })),
                }),
                ..Default::default()
            }),
            native: NativeBytes::new(),
            txo_ref: Some(query::TxoRef {
                hash: vec![0x11; 32].into(),
                index: 0,
            }),
        })
        .expect_err("native proto without bytes");
        assert!(error.to_string().contains("native reference script"));
    }

    #[test]
    fn parsed_output_rejects_short_policy_id() {
        let error = map_wire_utxo(ChainUtxo {
            parsed: Some(cardano::TxOutput {
                address: addr_bytes().into(),
                coin: Some(cardano::BigInt {
                    big_int: Some(cardano::big_int::BigInt::Int(1)),
                }),
                assets: vec![cardano::Multiasset {
                    policy_id: vec![0; 27].into(),
                    assets: Vec::new(),
                    redeemer: None,
                }],
                ..Default::default()
            }),
            native: NativeBytes::new(),
            txo_ref: Some(query::TxoRef {
                hash: vec![0x11; 32].into(),
                index: 0,
            }),
        })
        .expect_err("short policy id");

        assert!(error.to_string().contains("unexpected policy id length"));
    }

    #[test]
    fn duplicate_assets_fail() {
        let policy = vec![0u8; 28];
        let error = map_wire_utxo(ChainUtxo {
            parsed: Some(cardano::TxOutput {
                address: addr_bytes().into(),
                coin: Some(cardano::BigInt {
                    big_int: Some(cardano::big_int::BigInt::Int(1)),
                }),
                assets: vec![cardano::Multiasset {
                    policy_id: policy.clone().into(),
                    assets: vec![
                        cardano::Asset {
                            name: vec![1].into(),
                            quantity: Some(cardano::asset::Quantity::OutputCoin(cardano::BigInt {
                                big_int: Some(cardano::big_int::BigInt::Int(1)),
                            })),
                        },
                        cardano::Asset {
                            name: vec![1].into(),
                            quantity: Some(cardano::asset::Quantity::OutputCoin(cardano::BigInt {
                                big_int: Some(cardano::big_int::BigInt::Int(2)),
                            })),
                        },
                    ],
                    redeemer: None,
                }],
                ..Default::default()
            }),
            native: NativeBytes::new(),
            txo_ref: Some(query::TxoRef {
                hash: vec![0x11; 32].into(),
                index: 0,
            }),
        })
        .expect_err("duplicates");
        assert!(error.to_string().contains("duplicate asset"));
    }
}
