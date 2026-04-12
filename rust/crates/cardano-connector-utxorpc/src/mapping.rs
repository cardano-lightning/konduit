use anyhow::{Context, anyhow};
use cardano_sdk::{
    Address, Credential, Hash, Input, Network, Output, PlutusData, PlutusScript, PlutusVersion,
    Value, cbor,
};
use pallas_primitives::conway::TransactionOutput;
use utxorpc::spec::{cardano, query};

pub fn map_input(reference: &query::TxoRef) -> anyhow::Result<Input> {
    let hash: [u8; 32] = reference
        .hash
        .as_ref()
        .try_into()
        .map_err(|_| anyhow!("unexpected tx hash length: {}", reference.hash.len()))?;

    Ok(Input::new(hash.into(), u64::from(reference.index)))
}

pub fn map_output(utxo: utxorpc::ChainUtxo<cardano::TxOutput>) -> anyhow::Result<(Input, Output)> {
    let reference = utxo
        .txo_ref
        .ok_or_else(|| anyhow!("UTxO response missing txo_ref"))?;

    let input = map_input(&reference)?;
    let output = map_output_data(utxo.parsed, utxo.native)
        .with_context(|| format!("failed to map UTxO for {}", input))?;

    Ok((input, output))
}

pub fn map_output_data(
    parsed: Option<cardano::TxOutput>,
    native: utxorpc::NativeBytes,
) -> anyhow::Result<Output> {
    if !native.is_empty() {
        let output: TransactionOutput = cbor::decode(native.as_ref())
            .context("failed to decode native transaction output bytes")?;
        return Output::try_from(output).context("failed to convert native output");
    }

    let parsed =
        parsed.ok_or_else(|| anyhow!("UTxO response missing parsed output and native bytes"))?;
    map_parsed_output(parsed)
}

pub fn matches_payment(output: &Output, payment: &Credential) -> bool {
    output
        .address()
        .as_shelley()
        .is_some_and(|address| address.payment() == *payment)
}

pub fn matches_payment_and_delegation(
    output: &Output,
    payment: &Credential,
    delegation: &Credential,
) -> bool {
    output.address().as_shelley().is_some_and(|address| {
        address.payment() == *payment && address.delegation().as_ref() == Some(delegation)
    })
}

pub fn predicate_for_credentials(
    _network: Network,
    payment: &Credential,
    _delegation: Option<&Credential>,
) -> utxorpc::spec::query::UtxoPredicate {
    let payment_part: [u8; Credential::DIGEST_SIZE] = payment.into();

    utxorpc::spec::query::UtxoPredicate {
        r#match: Some(utxorpc::spec::query::AnyUtxoPattern {
            utxo_pattern: Some(
                utxorpc::spec::query::any_utxo_pattern::UtxoPattern::Cardano(
                    cardano::TxOutputPattern {
                        address: Some(cardano::AddressPattern {
                            exact_address: Default::default(),
                            payment_part: payment_part.to_vec().into(),
                            delegation_part: Default::default(),
                        }),
                        asset: None,
                    },
                ),
            ),
        }),
        not: vec![],
        all_of: vec![],
        any_of: vec![],
    }
}

fn map_parsed_output(parsed: cardano::TxOutput) -> anyhow::Result<Output> {
    let address = Address::try_from(parsed.address.as_ref())?;
    let coin = parsed
        .coin
        .ok_or_else(|| anyhow!("parsed UTxO output missing coin value"))?;
    let mut assets = Vec::new();

    for multiasset in parsed.assets {
        let policy_id = bytes28(multiasset.policy_id.as_ref(), "policy id")?;
        let mut values = Vec::new();

        for asset in multiasset.assets {
            let quantity = asset
                .quantity
                .ok_or_else(|| anyhow!("parsed asset missing quantity"))?;

            let amount = match quantity {
                cardano::asset::Quantity::OutputCoin(value) => big_int_to_u64(&value)?,
                cardano::asset::Quantity::MintCoin(_) => {
                    return Err(anyhow!("parsed UTxO asset used mint quantity"));
                }
            };

            values.push((asset.name.to_vec(), amount));
        }

        assets.push((policy_id.into(), values));
    }

    let mut output = Output::new(
        address,
        Value::new(big_int_to_u64(&coin)?).with_assets(assets),
    );

    if let Some(datum) = parsed.datum {
        if !datum.original_cbor.is_empty() {
            let data: PlutusData<'static> = cbor::decode(datum.original_cbor.as_ref())
                .context("failed to decode inline datum bytes")?;
            output = output.with_datum(data);
        } else if !datum.hash.is_empty() {
            output =
                output.with_datum_hash(Hash::from(bytes32(datum.hash.as_ref(), "datum hash")?));
        }
    }

    if let Some(script) = parsed.script {
        output = output.with_plutus_script(map_plutus_script(script)?);
    }

    Ok(output)
}

fn bytes28(bytes: &[u8], label: &str) -> anyhow::Result<[u8; 28]> {
    bytes
        .try_into()
        .map_err(|_| anyhow!("unexpected {label} length: {}", bytes.len()))
}

fn bytes32(bytes: &[u8], label: &str) -> anyhow::Result<[u8; 32]> {
    bytes
        .try_into()
        .map_err(|_| anyhow!("unexpected {label} length: {}", bytes.len()))
}

fn map_plutus_script(script: cardano::Script) -> anyhow::Result<PlutusScript> {
    match script.script {
        Some(cardano::script::Script::Native(_)) => {
            Err(anyhow!("unsupported native reference script"))
        }
        Some(cardano::script::Script::PlutusV1(bytes)) => {
            Ok(PlutusScript::new(PlutusVersion::V1, bytes.to_vec()))
        }
        Some(cardano::script::Script::PlutusV2(bytes)) => {
            Ok(PlutusScript::new(PlutusVersion::V2, bytes.to_vec()))
        }
        Some(cardano::script::Script::PlutusV3(bytes)) => {
            Ok(PlutusScript::new(PlutusVersion::V3, bytes.to_vec()))
        }
        None => Err(anyhow!("script payload missing")),
    }
}

fn big_int_to_u64(value: &cardano::BigInt) -> anyhow::Result<u64> {
    match &value.big_int {
        Some(cardano::big_int::BigInt::Int(value)) if *value >= 0 => Ok(*value as u64),
        Some(cardano::big_int::BigInt::Int(value)) => {
            Err(anyhow!("expected non-negative integer, got {value}"))
        }
        Some(cardano::big_int::BigInt::BigUInt(bytes)) => bytes_to_u64(bytes),
        Some(cardano::big_int::BigInt::BigNInt(_)) => Err(anyhow!(
            "expected non-negative integer, got negative big integer"
        )),
        None => Err(anyhow!("missing integer payload")),
    }
}

fn bytes_to_u64(bytes: &[u8]) -> anyhow::Result<u64> {
    if bytes.len() > 8 {
        return Err(anyhow!("expected non-negative integer that fits in u64"));
    }

    Ok(bytes
        .iter()
        .fold(0u64, |acc, byte| (acc << 8) | u64::from(*byte)))
}

#[cfg(test)]
mod tests {
    use super::{
        big_int_to_u64, map_output_data, matches_payment, matches_payment_and_delegation,
        predicate_for_credentials,
    };
    use cardano_sdk::{
        Address, Credential, Datum, Network, Output, PlutusData, PlutusVersion, Value,
        address_test, cbor::ToCbor, key_credential, plutus_script,
    };
    use utxorpc::{NativeBytes, spec::cardano::big_int::BigInt as BigIntValue};

    fn parsed_output(output: Output) -> utxorpc::spec::cardano::TxOutput {
        utxorpc::spec::cardano::TxOutput {
            address: Vec::<u8>::from(output.address()).into(),
            coin: Some(utxorpc::spec::cardano::BigInt {
                big_int: Some(BigIntValue::Int(output.value().lovelace() as i64)),
            }),
            assets: output
                .value()
                .assets()
                .iter()
                .map(|(policy, assets)| utxorpc::spec::cardano::Multiasset {
                    policy_id: Vec::from(policy.as_ref()).into(),
                    assets: assets
                        .iter()
                        .map(|(name, amount)| utxorpc::spec::cardano::Asset {
                            name: name.clone().into(),
                            quantity: Some(utxorpc::spec::cardano::asset::Quantity::OutputCoin(
                                utxorpc::spec::cardano::BigInt {
                                    big_int: Some(BigIntValue::Int(*amount as i64)),
                                },
                            )),
                        })
                        .collect(),
                    redeemer: None,
                })
                .collect(),
            datum: output.datum().map(|datum| match datum {
                Datum::Hash(hash) => utxorpc::spec::cardano::Datum {
                    hash: Vec::from(hash.as_ref()).into(),
                    payload: None,
                    original_cbor: Default::default(),
                },
                Datum::Inline(data) => utxorpc::spec::cardano::Datum {
                    hash: Default::default(),
                    payload: None,
                    original_cbor: data.to_cbor().into(),
                },
            }),
            script: output
                .script()
                .map(|script| utxorpc::spec::cardano::Script {
                    script: Some(match script.version() {
                        PlutusVersion::V1 => utxorpc::spec::cardano::script::Script::PlutusV1(
                            script.script().to_vec().into(),
                        ),
                        PlutusVersion::V2 => utxorpc::spec::cardano::script::Script::PlutusV2(
                            script.script().to_vec().into(),
                        ),
                        PlutusVersion::V3 => utxorpc::spec::cardano::script::Script::PlutusV3(
                            script.script().to_vec().into(),
                        ),
                    }),
                }),
        }
    }

    #[test]
    fn big_uint_overflow_is_rejected() {
        let value = utxorpc::spec::cardano::BigInt {
            big_int: Some(BigIntValue::BigUInt(vec![0; 9].into())),
        };

        let error = big_int_to_u64(&value).expect_err("overflow should fail");

        assert!(
            error
                .to_string()
                .contains("expected non-negative integer that fits in u64")
        );
    }

    #[test]
    fn payment_only_predicate_leaves_delegation_unset() {
        let payment = key_credential!("11111111111111111111111111111111111111111111111111111111");

        let predicate = predicate_for_credentials(Network::Preprod, &payment, None);
        let pattern = predicate.r#match.expect("match pattern").utxo_pattern;
        let utxorpc::spec::query::any_utxo_pattern::UtxoPattern::Cardano(pattern) =
            pattern.expect("cardano pattern");
        let address = pattern.address.expect("address pattern");

        assert_eq!(address.payment_part.len(), Credential::DIGEST_SIZE);
        assert!(address.delegation_part.is_empty());
    }

    #[test]
    fn payment_and_delegation_predicate_still_leaves_delegation_unset() {
        let payment = key_credential!("11111111111111111111111111111111111111111111111111111111");
        let delegation =
            key_credential!("22222222222222222222222222222222222222222222222222222222");

        let predicate = predicate_for_credentials(Network::Preprod, &payment, Some(&delegation));
        let pattern = predicate.r#match.expect("match pattern").utxo_pattern;
        let utxorpc::spec::query::any_utxo_pattern::UtxoPattern::Cardano(pattern) =
            pattern.expect("cardano pattern");
        let address = pattern.address.expect("address pattern");

        assert_eq!(address.payment_part.len(), Credential::DIGEST_SIZE);
        assert!(address.delegation_part.is_empty());
    }

    #[test]
    fn matches_payment_ignores_delegation() {
        let payment = key_credential!("11111111111111111111111111111111111111111111111111111111");
        let delegation =
            key_credential!("22222222222222222222222222222222222222222222222222222222");
        let address =
            Address::new(Network::Preprod.into(), payment.clone()).with_delegation(delegation);
        let output = cardano_sdk::Output::new(address.into(), cardano_sdk::Value::new(1_000_000));

        assert!(matches_payment(&output, &payment));
    }

    #[test]
    fn matches_payment_and_delegation_requires_exact_pair() {
        let payment = key_credential!("11111111111111111111111111111111111111111111111111111111");
        let delegation =
            key_credential!("22222222222222222222222222222222222222222222222222222222");
        let other_delegation =
            key_credential!("33333333333333333333333333333333333333333333333333333333");
        let address = Address::new(Network::Preprod.into(), payment.clone())
            .with_delegation(delegation.clone());
        let output = cardano_sdk::Output::new(address.into(), cardano_sdk::Value::new(1_000_000));

        assert!(matches_payment_and_delegation(
            &output,
            &payment,
            &delegation
        ));
        assert!(!matches_payment_and_delegation(
            &output,
            &payment,
            &other_delegation,
        ));
    }

    #[test]
    fn native_bytes_take_precedence_over_parsed_output() {
        let native_output = Output::new(
            address_test!(key_credential!(
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            ))
            .into(),
            Value::new(4_000_000),
        )
        .with_datum(PlutusData::integer(7));
        let parsed = parsed_output(Output::new(
            address_test!(key_credential!(
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
            ))
            .into(),
            Value::new(1_000_000),
        ));

        let mapped = map_output_data(Some(parsed), NativeBytes::from(native_output.to_cbor()))
            .expect("native bytes should win");

        assert_eq!(mapped, native_output);
    }

    #[test]
    fn parsed_output_is_used_when_native_bytes_are_absent() {
        let expected = Output::new(
            address_test!(key_credential!(
                "cccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
            ))
            .into(),
            Value::new(5_000_000).with_assets([(
                cardano_sdk::hash!("279c909f348e533da5808898f87f9a14bb2c3dfbbacccd631d927a3f"),
                [(b"SNEK".to_vec(), 12)],
            )]),
        )
        .with_datum(PlutusData::bytes(b"hello"))
        .with_plutus_script(plutus_script!(
            PlutusVersion::V3,
            "5101010023259800a518a4d136564004ae69"
        ));

        let mapped = map_output_data(Some(parsed_output(expected.clone())), NativeBytes::new())
            .expect("parsed fallback should map");

        assert_eq!(mapped, expected);
    }

    #[test]
    fn parsed_output_requires_coin_value() {
        let error = map_output_data(
            Some(utxorpc::spec::cardano::TxOutput {
                address: Vec::<u8>::from(&address_test!(key_credential!(
                    "dddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
                )))
                .into(),
                coin: None,
                assets: Vec::new(),
                datum: None,
                script: None,
            }),
            NativeBytes::new(),
        )
        .expect_err("missing coin should fail");

        assert!(error.to_string().contains("missing coin value"));
    }

    #[test]
    fn parsed_output_rejects_native_reference_scripts() {
        let error = map_output_data(
            Some(utxorpc::spec::cardano::TxOutput {
                address: Vec::<u8>::from(&address_test!(key_credential!(
                    "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
                )))
                .into(),
                coin: Some(utxorpc::spec::cardano::BigInt {
                    big_int: Some(BigIntValue::Int(1_000_000)),
                }),
                assets: Vec::new(),
                datum: None,
                script: Some(utxorpc::spec::cardano::Script {
                    script: Some(utxorpc::spec::cardano::script::Script::Native(
                        utxorpc::spec::cardano::NativeScript {
                            native_script: Some(
                                utxorpc::spec::cardano::native_script::NativeScript::InvalidBefore(
                                    123,
                                ),
                            ),
                        },
                    )),
                }),
            }),
            NativeBytes::new(),
        )
        .expect_err("native reference scripts should fail");

        assert!(
            error
                .to_string()
                .contains("unsupported native reference script")
        );
    }
}
