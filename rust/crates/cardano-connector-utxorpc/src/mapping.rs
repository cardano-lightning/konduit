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

pub fn predicate_for_credentials(
    _network: Network,
    payment: &Credential,
    delegation: Option<&Credential>,
) -> utxorpc::spec::query::UtxoPredicate {
    let payment_part: [u8; Credential::DIGEST_SIZE] = payment.into();
    let delegation_part = delegation.map(<[u8; Credential::DIGEST_SIZE]>::from);

    utxorpc::spec::query::UtxoPredicate {
        r#match: Some(utxorpc::spec::query::AnyUtxoPattern {
            utxo_pattern: Some(
                utxorpc::spec::query::any_utxo_pattern::UtxoPattern::Cardano(
                    cardano::TxOutputPattern {
                        address: Some(cardano::AddressPattern {
                            exact_address: Default::default(),
                            payment_part: payment_part.to_vec().into(),
                            delegation_part: delegation_part
                                .map(|part| part.to_vec().into())
                                .unwrap_or_default(),
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
    use super::{big_int_to_u64, matches_payment, predicate_for_credentials};
    use cardano_sdk::{Address, Credential, Network, key_credential};
    use utxorpc::spec::cardano::big_int::BigInt as BigIntValue;

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
    fn matches_payment_ignores_delegation() {
        let payment = key_credential!("11111111111111111111111111111111111111111111111111111111");
        let delegation =
            key_credential!("22222222222222222222222222222222222222222222222222222222");
        let address =
            Address::new(Network::Preprod.into(), payment.clone()).with_delegation(delegation);
        let output = cardano_sdk::Output::new(address.into(), cardano_sdk::Value::new(1_000_000));

        assert!(matches_payment(&output, &payment));
    }
}
